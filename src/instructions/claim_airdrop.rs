use core::mem::transmute;

use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

use crate::{
    errors::AirdropProgramError,
    states::{AirdropState, ClaimStatus},
    utils::{
        create_airdrop_leaf, load_acc_mut_unchecked, load_acc_unchecked, verify_merkle_proof,
        DataLen,
    },
};

pub struct ClaimAirdropAccounts<'info> {
    pub airdrop_state: &'info AccountInfo,
    pub signer: &'info AccountInfo,
    pub user_claim: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for ClaimAirdropAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [airdrop_state, signer, user_claim, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // verify airdrop_state
        if !airdrop_state.is_writable() || airdrop_state.data_len() == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        if !user_claim.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !user_claim.data_is_empty() {
            return Err(AirdropProgramError::AccountAlreadyClaimed.into());
        }

        Ok(ClaimAirdropAccounts {
            airdrop_state,
            signer,
            user_claim,
        })
    }
}

#[repr(C, packed)]
pub struct ClaimAirdropInstructionData {
    pub amount: u64,
    pub leaf_index: u64,
    pub bump: u8,
    pub proof_len: u8,
}

impl DataLen for ClaimAirdropInstructionData {
    const LEN: usize = core::mem::size_of::<ClaimAirdropInstructionData>();
}

impl<'info> TryFrom<&'info [u8]> for ClaimAirdropInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        let fixed_data = &data[..Self::LEN];

        Ok(unsafe {
            transmute(
                TryInto::<[u8; Self::LEN]>::try_into(fixed_data)
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            )
        })
    }
}

pub struct ClaimAirdrop<'info> {
    pub accounts: ClaimAirdropAccounts<'info>,
    pub instruction_data: ClaimAirdropInstructionData,
    pub proof_data: &'info [[u8; 32]], // Slice reference to proof elements
}

impl<'info> ClaimAirdrop<'info> {
    /// Parse instruction data và proof từ raw bytes
    pub fn parse_from_data(
        data: &'info [u8],
        accounts: &'info [AccountInfo],
    ) -> Result<Self, ProgramError> {
        let accounts = ClaimAirdropAccounts::try_from(accounts)?;
        let instruction_data = ClaimAirdropInstructionData::try_from(data)?;

        // calculate offset for proof data
        let proof_offset = ClaimAirdropInstructionData::LEN;
        let proof_len = instruction_data.proof_len as usize;

        // check data length
        let expected_len = proof_offset + (proof_len * 32);
        if data.len() != expected_len {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Parse proof data as slice of [u8; 32]
        let proof_bytes = &data[proof_offset..];
        let proof_data = unsafe {
            core::slice::from_raw_parts(proof_bytes.as_ptr() as *const [u8; 32], proof_len)
        };

        Ok(ClaimAirdrop {
            accounts,
            instruction_data,
            proof_data,
        })
    }

    /// Get proof as slice
    pub fn get_proof(&self) -> &[[u8; 32]] {
        self.proof_data
    }
}

impl<'info> TryFrom<(&'info [u8], &'info [AccountInfo])> for ClaimAirdrop<'info> {
    type Error = ProgramError;

    fn try_from(
        (data, accounts): (&'info [u8], &'info [AccountInfo]),
    ) -> Result<Self, Self::Error> {
        Self::parse_from_data(data, accounts)
    }
}

impl<'info> ClaimAirdrop<'info> {
    pub const DISCRIMINATOR: &'info u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        // Get proof data
        let proof = self.get_proof();
        let amount = self.instruction_data.amount;
        let leaf_index = self.instruction_data.leaf_index;

        // Create leaf hash
        let claimer = *self.accounts.signer.key();
        let leaf = create_airdrop_leaf(&claimer, amount, 0);
        let airdrop_state = unsafe {
            load_acc_unchecked::<AirdropState>(self.accounts.airdrop_state.borrow_data_unchecked())
        }?;
        let merkle_root = airdrop_state.merkle_root;

        // Verify merkle proof
        let is_valid = verify_merkle_proof(&leaf, proof, leaf_index, &merkle_root);

        if !is_valid {
            return Err(AirdropProgramError::InvalidProof.into());
        }

        ClaimStatus::validate_pda(
            self.accounts.user_claim.key(),
            self.accounts.airdrop_state.key(),
            self.accounts.signer.key(),
            self.instruction_data.bump,
        )?;

        // // init user_claim to avoid double claims
        {
            let bump_binding = [self.instruction_data.bump];
            let seed = [
                Seed::from(ClaimStatus::SEED),
                Seed::from(self.accounts.airdrop_state.key().as_ref()),
                Seed::from(self.accounts.signer.key().as_ref()),
                Seed::from(&bump_binding),
            ];
            let signer_seeds = Signer::from(&seed);

            pinocchio_system::instructions::CreateAccount {
                from: self.accounts.signer,
                to: self.accounts.user_claim,
                space: ClaimStatus::LEN as u64,
                lamports: Rent::get()?.minimum_balance(ClaimStatus::LEN),
                owner: &crate::ID,
            }
            .invoke_signed(&[signer_seeds])?;

            let mut data: pinocchio::account_info::RefMut<'_, [u8]> =
                self.accounts.user_claim.try_borrow_mut_data()?;
            let user_claim = unsafe { load_acc_mut_unchecked::<ClaimStatus>(&mut data) }?;

            user_claim.bump = [self.instruction_data.bump];
        }

        {
            *self.accounts.airdrop_state.try_borrow_mut_lamports()? -= amount;
            *self.accounts.signer.try_borrow_mut_lamports()? += amount;
        }

        {
            let airdrop_state = unsafe {
                load_acc_mut_unchecked::<AirdropState>(
                    self.accounts.airdrop_state.borrow_mut_data_unchecked(),
                )
            }?;
            airdrop_state.amount_claimed = (u64::from_le_bytes(airdrop_state.amount_claimed)
                .saturating_add(amount))
            .to_be_bytes();
        }

        Ok(())
    }
}
