use core::mem::transmute;

use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

use crate::{
    errors::AirdropProgramError,
    states::AirdropState,
    utils::{load_acc_mut_unchecked, load_acc_unchecked, DataLen},
};

pub struct UpdateMerkleRootAccounts<'info> {
    pub airdrop_state: &'info AccountInfo,
    pub authority: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for UpdateMerkleRootAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [airdrop_state, authority, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // verify airdrop_state
        if !airdrop_state.is_writable() || airdrop_state.data_is_empty() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(UpdateMerkleRootAccounts {
            airdrop_state,
            authority,
        })
    }
}

#[repr(C, packed)]
pub struct UpdateMerkleRootInstructionData {
    pub new_merkle_root: [u8; 32],
    pub additional_amount: u64,
}

impl DataLen for UpdateMerkleRootInstructionData {
    const LEN: usize = core::mem::size_of::<UpdateMerkleRootInstructionData>();
}

impl<'info> TryFrom<&'info [u8]> for UpdateMerkleRootInstructionData {
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

pub struct UpdateMerkleRootAirdrop<'info> {
    pub accounts: UpdateMerkleRootAccounts<'info>,
    pub instruction_data: UpdateMerkleRootInstructionData,
}

impl<'info> TryFrom<(&'info [u8], &'info [AccountInfo])> for UpdateMerkleRootAirdrop<'info> {
    type Error = ProgramError;

    fn try_from(
        (data, accounts): (&'info [u8], &'info [AccountInfo]),
    ) -> Result<Self, Self::Error> {
        let accounts = UpdateMerkleRootAccounts::try_from(accounts)?;
        let instruction_data = UpdateMerkleRootInstructionData::try_from(data)?;

        Ok(UpdateMerkleRootAirdrop {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> UpdateMerkleRootAirdrop<'info> {
    pub const DISCRIMINATOR: &'info u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        {
            let airdrop_state_data = unsafe {
                load_acc_unchecked::<AirdropState>(
                    &mut self.accounts.airdrop_state.borrow_data_unchecked(),
                )?
            };

            if self
                .accounts
                .authority
                .key()
                .ne(&airdrop_state_data.authority)
            {
                return Err(AirdropProgramError::Unauthorized.into());
            }
        }

        {
            let data = unsafe { self.accounts.airdrop_state.borrow_mut_data_unchecked() };
            let airdrop_state_data = unsafe { load_acc_mut_unchecked::<AirdropState>(data)? };

            airdrop_state_data.merkle_root = self.instruction_data.new_merkle_root;

            if self.instruction_data.additional_amount > 0 {
                pinocchio_system::instructions::Transfer {
                    from: self.accounts.authority,
                    to: self.accounts.airdrop_state,
                    lamports: self.instruction_data.additional_amount,
                }
                .invoke()?;

                airdrop_state_data.airdrop_amount =
                    u64::from_le_bytes(airdrop_state_data.airdrop_amount)
                        .saturating_add(self.instruction_data.additional_amount)
                        .to_le_bytes();
            }
        }

        Ok(())
    }
}
