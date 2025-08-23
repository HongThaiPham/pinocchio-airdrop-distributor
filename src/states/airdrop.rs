use pinocchio::{
    program_error::ProgramError,
    pubkey::{self, Pubkey},
};

use crate::utils::DataLen;

#[repr(C)]
pub struct AirdropState {
    /// The Merkle root of the airdrop (32 bytes)
    pub merkle_root: [u8; 32],
    /// The authority allowed to update the merkle root
    pub authority: Pubkey,
    /// Total SOL allocated for this airdrop (in lamports)
    pub airdrop_amount: [u8; 8],
    /// Total SOL claimed so far (in lamports)
    pub amount_claimed: [u8; 8],
    /// Bump seed for the PDA
    pub bump: [u8; 1],
}

impl DataLen for AirdropState {
    const LEN: usize = core::mem::size_of::<AirdropState>();
}

impl AirdropState {
    pub const SEED: &'static [u8] = b"merkle_tree";

    pub fn validate_pda(target: &Pubkey, bump: u8) -> Result<(), ProgramError> {
        let seed_with_bump = &[Self::SEED, &[bump]];
        let expected = pubkey::create_program_address(seed_with_bump, &crate::ID)?;
        if expected != *target {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}
