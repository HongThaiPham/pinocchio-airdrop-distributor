use pinocchio::program_error::ProgramError;
use solana_nostd_keccak::hash;
pub trait DataLen {
    const LEN: usize;
}

#[inline(always)]
pub unsafe fn load_acc_unchecked<T: DataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

#[inline(always)]
pub unsafe fn load_acc_mut_unchecked<T: DataLen>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&mut *(bytes.as_mut_ptr() as *mut T))
}

#[inline(always)]
pub unsafe fn load_ix_data<T: DataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

pub unsafe fn to_bytes<T: DataLen>(data: &T) -> &[u8] {
    core::slice::from_raw_parts(data as *const T as *const u8, T::LEN)
}

pub unsafe fn to_mut_bytes<T: DataLen>(data: &mut T) -> &mut [u8] {
    core::slice::from_raw_parts_mut(data as *mut T as *mut u8, T::LEN)
}

/// Helper function to hash two 32-byte arrays together
#[inline(always)]
pub fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hash_input = [0u8; 64];
    hash_input[..32].copy_from_slice(left);
    hash_input[32..].copy_from_slice(right);
    hash(&hash_input)
}

/// Optimized helper function to verify Merkle proof
pub fn verify_merkle_proof(
    leaf: &[u8; 32],
    proof: &[[u8; 32]],
    leaf_index: u64,
    expected_root: &[u8; 32],
) -> bool {
    // Early return for empty proof
    if proof.is_empty() {
        return leaf == expected_root;
    }

    let mut computed_hash = *leaf;
    let mut index = leaf_index;

    for proof_element in proof.iter() {
        computed_hash = if index & 1 == 0 {
            // Current node is left child
            hash_pair(&computed_hash, proof_element)
        } else {
            // Current node is right child
            hash_pair(proof_element, &computed_hash)
        };
        index >>= 1; // Equivalent to index /= 2 but faster
    }

    computed_hash == *expected_root
}

/// Create a leaf hash from recipient address and amount
#[inline(always)]
pub fn create_airdrop_leaf(recipient: &[u8; 32], amount: u64, is_claimed: u8) -> [u8; 32] {
    let mut hash_input = [0u8; 41]; // 32 bytes for address + 8 bytes for amount + 1 byte for is_claimed
    hash_input[..32].copy_from_slice(recipient);
    hash_input[32..40].copy_from_slice(&amount.to_le_bytes());
    hash_input[40] = is_claimed;
    hash(&hash_input)
}
