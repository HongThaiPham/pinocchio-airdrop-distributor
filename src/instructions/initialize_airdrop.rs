use core::mem::transmute;

use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

use crate::{
    states::AirdropState,
    utils::{load_acc_mut_unchecked, DataLen},
};

pub struct InitializeAirdropAccounts<'info> {
    pub airdrop_state: &'info AccountInfo,
    pub authority: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for InitializeAirdropAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [airdrop_state, authority, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // verify airdrop_state
        if !airdrop_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !airdrop_state.data_is_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        Ok(InitializeAirdropAccounts {
            airdrop_state,
            authority,
        })
    }
}

#[repr(C, packed)]
pub struct InitializeAirdropInstructionData {
    pub merkle_root: [u8; 32],
    pub amount: u64,
    pub bump: u8,
}

impl DataLen for InitializeAirdropInstructionData {
    const LEN: usize = core::mem::size_of::<InitializeAirdropInstructionData>();
}

impl<'info> TryFrom<&'info [u8]> for InitializeAirdropInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(unsafe {
            transmute(
                TryInto::<[u8; Self::LEN]>::try_into(data)
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            )
        })
    }
}

pub struct InitializeAirdrop<'info> {
    pub accounts: InitializeAirdropAccounts<'info>,
    pub instruction_data: InitializeAirdropInstructionData,
}

impl<'info> TryFrom<(&'info [u8], &'info [AccountInfo])> for InitializeAirdrop<'info> {
    type Error = ProgramError;

    fn try_from(
        (data, accounts): (&'info [u8], &'info [AccountInfo]),
    ) -> Result<Self, Self::Error> {
        let accounts = InitializeAirdropAccounts::try_from(accounts)?;
        let instruction_data = InitializeAirdropInstructionData::try_from(data)?;

        Ok(InitializeAirdrop {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> InitializeAirdrop<'info> {
    pub const DISCRIMINATOR: &'info u8 = &0;

    pub fn process(&mut self) -> ProgramResult {
        AirdropState::validate_pda(
            self.accounts.airdrop_state.key(),
            self.instruction_data.bump,
        )?;

        {
            // create and init airdrop state account
            let bump_binding = [self.instruction_data.bump];
            let seed = [Seed::from(AirdropState::SEED), Seed::from(&bump_binding)];
            let signer_seeds = Signer::from(&seed);

            pinocchio_system::instructions::CreateAccount {
                from: self.accounts.authority,
                to: self.accounts.airdrop_state,
                space: AirdropState::LEN as u64,
                lamports: Rent::get()?.minimum_balance(AirdropState::LEN),
                owner: &crate::ID,
            }
            .invoke_signed(&[signer_seeds])?;

            let mut data: pinocchio::account_info::RefMut<'_, [u8]> =
                self.accounts.airdrop_state.try_borrow_mut_data()?;
            let airdrop_state = unsafe { load_acc_mut_unchecked::<AirdropState>(&mut data) }?;

            airdrop_state.merkle_root = self.instruction_data.merkle_root;
            airdrop_state.authority = *self.accounts.authority.key();
            airdrop_state.bump = [self.instruction_data.bump];
            airdrop_state.airdrop_amount = self.instruction_data.amount.to_le_bytes();
            airdrop_state.amount_claimed = 0u64.to_le_bytes();
        }

        {
            // transfer sol to airdrop_state
            pinocchio_system::instructions::Transfer {
                from: self.accounts.authority,
                to: self.accounts.airdrop_state,
                lamports: self.instruction_data.amount,
            }
            .invoke()?;
        }
        Ok(())
    }
}
