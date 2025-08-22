use core::mem::transmute;

use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

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

impl InitializeAirdropInstructionData {
    pub const LEN: usize = core::mem::size_of::<InitializeAirdropInstructionData>();
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
        Ok(())
    }
}
