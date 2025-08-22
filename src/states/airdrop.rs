use pinocchio::pubkey::Pubkey;

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
