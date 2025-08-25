use core::mem::transmute;

use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

use crate::utils::DataLen;

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
        if !airdrop_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !airdrop_state.data_is_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        Ok(UpdateMerkleRootAccounts {
            airdrop_state,
            authority,
        })
    }
}

#[repr(C, packed)]
pub struct UpdateMerkleRootInstructionData {
    pub merkle_root: [u8; 32],
    pub amount: u64,
    pub bump: u8,
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
        Ok(())
    }
}
