use pinocchio::{
    program_error::ProgramError,
    pubkey::{self, Pubkey},
};

use crate::utils::DataLen;

#[repr(C)]
pub struct ClaimStatus {
    pub bump: [u8; 1],
}
impl DataLen for ClaimStatus {
    const LEN: usize = core::mem::size_of::<ClaimStatus>();
}

impl ClaimStatus {
    pub const SEED: &'static [u8] = b"claim";

    pub fn validate_pda(
        target: &Pubkey,
        airdrop: &Pubkey,
        claimer: &Pubkey,
        bump: u8,
    ) -> Result<(), ProgramError> {
        let seed_with_bump = &[Self::SEED, airdrop.as_ref(), claimer.as_ref(), &[bump]];
        let expected = pubkey::create_program_address(seed_with_bump, &crate::ID)?;
        if expected != *target {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}
