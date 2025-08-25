use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::instructions::{ClaimAirdrop, InitializeAirdrop};

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        Some((InitializeAirdrop::DISCRIMINATOR, data)) => {
            InitializeAirdrop::try_from((data, accounts))?.process()
        }
        Some((ClaimAirdrop::DISCRIMINATOR, data)) => {
            ClaimAirdrop::try_from((data, accounts))?.process()
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
