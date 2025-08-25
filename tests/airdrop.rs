#[cfg(test)]
mod tests_airdrop_distributor {
    use mollusk_svm::{
        result::{Check, ProgramResult},
        Mollusk,
    };

    use pinocchio_airdrop_distributor::{
        instructions::{
            ClaimAirdropInstructionData, InitializeAirdropInstructionData,
            UpdateMerkleRootInstructionData,
        },
        states::{AirdropState, ClaimStatus},
        utils::{to_bytes, DataLen},
        *,
    };
    use solana_sdk::{
        account::{Account, AccountSharedData, ReadableAccount},
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        program_error::ProgramError,
        pubkey::Pubkey,
    };

    pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(ID);

    fn create_merkle_root(airdrop_data: &[(Pubkey, u64)]) -> [u8; 32] {
        use pinocchio_airdrop_distributor::utils::{create_airdrop_leaf, hash_pair};

        if airdrop_data.is_empty() {
            return [0u8; 32];
        }

        // Tạo leaves từ airdrop data
        let mut leaves: Vec<[u8; 32]> = airdrop_data
            .iter()
            .map(|(pubkey, amount)| create_airdrop_leaf(&pubkey.to_bytes(), *amount, 0))
            .collect();

        // Nếu chỉ có 1 leaf, return leaf đó làm root
        if leaves.len() == 1 {
            return leaves[0];
        }

        // Build merkle tree bottom-up
        while leaves.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in leaves.chunks(2) {
                if chunk.len() == 2 {
                    // Hash hai leaf với nhau
                    let parent = hash_pair(&chunk[0], &chunk[1]);
                    next_level.push(parent);
                } else {
                    // Nếu số lượng lẻ, duplicate leaf cuối
                    let parent = hash_pair(&chunk[0], &chunk[0]);
                    next_level.push(parent);
                }
            }

            leaves = next_level;
        }

        leaves[0]
    }

    fn create_merkle_proof(airdrop_data: &[(Pubkey, u64)], target_index: usize) -> Vec<[u8; 32]> {
        use pinocchio_airdrop_distributor::utils::{create_airdrop_leaf, hash_pair};

        if target_index >= airdrop_data.len() {
            return vec![];
        }

        // Tạo leaves
        let mut leaves: Vec<[u8; 32]> = airdrop_data
            .iter()
            .map(|(pubkey, amount)| create_airdrop_leaf(&pubkey.to_bytes(), *amount, 0))
            .collect();

        let mut proof = Vec::new();
        let mut current_index = target_index;

        // Build proof path từ leaf đến root
        while leaves.len() > 1 {
            let mut next_level = Vec::new();

            for (i, chunk) in leaves.chunks(2).enumerate() {
                if chunk.len() == 2 {
                    // Nếu current_index nằm trong chunk này
                    if current_index / 2 == i {
                        if current_index % 2 == 0 {
                            // Current node là left child, add right sibling
                            proof.push(chunk[1]);
                        } else {
                            // Current node là right child, add left sibling
                            proof.push(chunk[0]);
                        }
                    }

                    let parent = hash_pair(&chunk[0], &chunk[1]);
                    next_level.push(parent);
                } else {
                    // Odd number case
                    let parent = hash_pair(&chunk[0], &chunk[0]);
                    next_level.push(parent);
                }
            }

            leaves = next_level;
            current_index /= 2;
        }

        proof
    }

    fn get_mollusk() -> Mollusk {
        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/pinocchio_airdrop_distributor");
        mollusk
    }

    #[test]
    fn test_hash_functions() {
        use pinocchio_airdrop_distributor::utils::{
            create_airdrop_leaf, hash_pair, verify_merkle_proof,
        };
        use solana_sdk::keccak;

        // Test hash_pair consistency
        let left = [1u8; 32];
        let right = [2u8; 32];

        let hash1 = hash_pair(&left, &right);

        // Test with reference implementation
        let mut reference_input = Vec::new();
        reference_input.extend_from_slice(&left);
        reference_input.extend_from_slice(&right);
        let reference_hash = keccak::hash(&reference_input);

        assert_eq!(hash1, reference_hash.to_bytes());
        println!("✅ hash_pair function produces correct result");

        // Test create_airdrop_leaf consistency
        let recipient = [42u8; 32];
        let amount = 1000u64;

        let leaf1 = create_airdrop_leaf(&recipient, amount, 0);

        let mut reference_input = Vec::new();
        reference_input.extend_from_slice(&recipient);
        reference_input.extend_from_slice(&amount.to_le_bytes());
        reference_input.push(0u8); // is_claimed = 0
        let reference_leaf = keccak::hash(&reference_input);

        assert_eq!(leaf1, reference_leaf.to_bytes());
        println!("✅ create_airdrop_leaf function produces correct result");

        // Test simple merkle proof
        let leaf1 = [1u8; 32];
        let leaf2 = [2u8; 32];
        let root = hash_pair(&leaf1, &leaf2);

        let proof = vec![leaf2];
        let is_valid = verify_merkle_proof(&leaf1, &proof, 0, &root);
        assert!(is_valid);

        println!("✅ All hash functions work correctly");
    }

    #[test]
    fn init_airdrop_state() {
        let mollusk = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let maker_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let (airdrop_address, bump) =
            Pubkey::find_program_address(&[AirdropState::SEED], &PROGRAM_ID);

        let airdrop_account = Account::new(0, 0, &system_program);

        let airdrop_recipients = vec![
            (Pubkey::new_unique(), 100_000_000u64),
            (Pubkey::new_unique(), 200_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
        ];
        let merkle_root = create_merkle_root(&airdrop_recipients);

        let amount = airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let ix_data = InitializeAirdropInstructionData {
            merkle_root,
            amount,
            bump,
        };

        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(AirdropState::LEN);

        let mut data = vec![0];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(airdrop_address, false),
                AccountMeta::new(maker, true),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let result: mollusk_svm::result::InstructionResult = mollusk
            .process_and_validate_instruction(
                &instruction,
                &[
                    (airdrop_address, airdrop_account.clone()),
                    (maker, maker_account.clone()),
                    (system_program, system_account.clone()),
                ],
                &[
                    Check::success(),
                    Check::account(&airdrop_address).owner(&PROGRAM_ID).build(),
                    Check::account(&airdrop_address)
                        .lamports(amount + lamport_for_rent)
                        .build(),
                ],
            );

        // check balance of aidrop_state

        let airdrop_account = result.get_account(&airdrop_address).unwrap();
        print!("{}", airdrop_account.lamports());
        assert!(airdrop_account.lamports() >= amount);
        assert!(result.program_result == ProgramResult::Success);
    }

    #[test]
    fn claim_airdrop_success() {
        let mollusk = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let _maker_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let claimer = Pubkey::new_from_array([0x03; 32]);
        let claimer_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let airdrop_recipients = vec![
            (Pubkey::new_unique(), 100_000_000u64),
            (Pubkey::new_unique(), 200_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
            (claimer, 50_000_000u64),
            (Pubkey::new_unique(), 75_000_000u64),
            (Pubkey::new_unique(), 125_000_000u64),
        ];
        let merkle_root = create_merkle_root(&airdrop_recipients);
        let amount: u64 = airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let (airdrop_address, airdrop_account_bump) =
            Pubkey::find_program_address(&[AirdropState::SEED], &PROGRAM_ID);

        let airdrop_account_data = AirdropState {
            authority: maker.to_bytes(),
            merkle_root,
            airdrop_amount: amount.to_le_bytes(),
            amount_claimed: 0u64.to_le_bytes(),
            bump: [airdrop_account_bump],
        };
        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(AirdropState::LEN);

        let mut airdrop_account =
            AccountSharedData::new(lamport_for_rent + amount, AirdropState::LEN, &PROGRAM_ID);

        airdrop_account
            .set_data_from_slice(unsafe { to_bytes::<AirdropState>(&airdrop_account_data) });

        let leaf_index = 3;
        let proof = create_merkle_proof(&airdrop_recipients, leaf_index as usize);

        let (user_claim_address, user_claim_account_bump) = Pubkey::find_program_address(
            &[
                ClaimStatus::SEED,
                airdrop_address.as_ref(),
                claimer.as_ref(),
            ],
            &PROGRAM_ID,
        );

        let user_claim_account = Account::new(0, 0, &system_program);

        let ix_data = ClaimAirdropInstructionData {
            amount: airdrop_recipients[leaf_index].1,
            leaf_index: leaf_index as u64,
            proof_len: proof.len() as u8,
            bump: user_claim_account_bump,
        };

        let mut data = vec![1];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        // add proof to data
        for proof_element in &proof {
            data.extend_from_slice(proof_element);
        }

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(airdrop_address, false),
                AccountMeta::new(claimer, true),
                AccountMeta::new(user_claim_address, false),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let result: mollusk_svm::result::InstructionResult = mollusk
            .process_and_validate_instruction(
                &instruction,
                &[
                    (airdrop_address, airdrop_account.into()),
                    (claimer, claimer_account.into()),
                    (user_claim_address, user_claim_account.into()),
                    (system_program, system_account.into()),
                ],
                &[
                    Check::success(),
                    Check::account(&airdrop_address).owner(&PROGRAM_ID).build(),
                    Check::account(&user_claim_address)
                        .owner(&PROGRAM_ID)
                        .build(),
                    Check::account(&airdrop_address)
                        .lamports(amount + lamport_for_rent - airdrop_recipients[leaf_index].1)
                        .build(),
                    // Check::account(&claimer)
                    //     .lamports(1 * LAMPORTS_PER_SOL + airdrop_recipients[leaf_index].1)
                    //     .build(),
                ],
            );
        assert!(result.program_result == ProgramResult::Success);
    }

    #[test]
    fn claim_airdrop_failt_with_invalid_proof() {
        let mollusk = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let _maker_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let claimer = Pubkey::new_from_array([0x04; 32]);
        let claimer_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let airdrop_recipients = vec![
            (Pubkey::new_unique(), 100_000_000u64),
            (Pubkey::new_unique(), 200_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
            (Pubkey::new_unique(), 50_000_000u64),
            (Pubkey::new_unique(), 75_000_000u64),
            (Pubkey::new_unique(), 125_000_000u64),
        ];
        let merkle_root = create_merkle_root(&airdrop_recipients);
        let amount: u64 = airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let (airdrop_address, airdrop_account_bump) =
            Pubkey::find_program_address(&[AirdropState::SEED], &PROGRAM_ID);

        let airdrop_account_data = AirdropState {
            authority: maker.to_bytes(),
            merkle_root,
            airdrop_amount: amount.to_le_bytes(),
            amount_claimed: 0u64.to_le_bytes(),
            bump: [airdrop_account_bump],
        };
        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(AirdropState::LEN);

        let mut airdrop_account =
            AccountSharedData::new(lamport_for_rent + amount, AirdropState::LEN, &PROGRAM_ID);

        airdrop_account
            .set_data_from_slice(unsafe { to_bytes::<AirdropState>(&airdrop_account_data) });

        let leaf_index = 3;
        let proof = create_merkle_proof(&airdrop_recipients, leaf_index as usize);

        let (user_claim_address, user_claim_account_bump) = Pubkey::find_program_address(
            &[
                ClaimStatus::SEED,
                airdrop_address.as_ref(),
                claimer.as_ref(),
            ],
            &PROGRAM_ID,
        );

        let user_claim_account = Account::new(0, 0, &system_program);

        let ix_data = ClaimAirdropInstructionData {
            amount: airdrop_recipients[leaf_index].1,
            leaf_index: leaf_index as u64,
            proof_len: proof.len() as u8,
            bump: user_claim_account_bump,
        };
        let mut data = vec![1];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        // add proof to data
        for proof_element in &proof {
            data.extend_from_slice(proof_element);
        }

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(airdrop_address, false),
                AccountMeta::new(claimer, true),
                AccountMeta::new(user_claim_address, false),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let result: mollusk_svm::result::InstructionResult = mollusk
            .process_and_validate_instruction(
                &instruction,
                &[
                    (airdrop_address, airdrop_account.into()),
                    (claimer, claimer_account.into()),
                    (user_claim_address, user_claim_account.into()),
                    (system_program, system_account.into()),
                ],
                &[
                    Check::err(ProgramError::Custom(0)), // invalid_proof
                    Check::account(&airdrop_address).owner(&PROGRAM_ID).build(),
                    Check::account(&airdrop_address)
                        .lamports(amount + lamport_for_rent)
                        .build(),
                ],
            );
        assert!(result.program_result == ProgramResult::Failure(ProgramError::Custom(0)));
    }

    #[test]
    fn claim_airdrop_failt_when_double_claim() {
        let mollusk = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let _maker_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let claimer = Pubkey::new_from_array([0x03; 32]);
        let claimer_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let airdrop_recipients = vec![
            (Pubkey::new_unique(), 100_000_000u64),
            (Pubkey::new_unique(), 200_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
            (Pubkey::new_unique(), 50_000_000u64),
            (Pubkey::new_unique(), 75_000_000u64),
            (Pubkey::new_unique(), 125_000_000u64),
        ];
        let merkle_root = create_merkle_root(&airdrop_recipients);
        let amount: u64 = airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let (airdrop_address, airdrop_account_bump) =
            Pubkey::find_program_address(&[AirdropState::SEED], &PROGRAM_ID);

        let airdrop_account_data = AirdropState {
            authority: maker.to_bytes(),
            merkle_root,
            airdrop_amount: amount.to_le_bytes(),
            amount_claimed: 0u64.to_le_bytes(),
            bump: [airdrop_account_bump],
        };
        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(AirdropState::LEN);

        let mut airdrop_account =
            AccountSharedData::new(lamport_for_rent + amount, AirdropState::LEN, &PROGRAM_ID);

        airdrop_account
            .set_data_from_slice(unsafe { to_bytes::<AirdropState>(&airdrop_account_data) });

        let leaf_index = 3;
        let proof = create_merkle_proof(&airdrop_recipients, leaf_index as usize);

        let (user_claim_address, user_claim_account_bump) = Pubkey::find_program_address(
            &[
                ClaimStatus::SEED,
                airdrop_address.as_ref(),
                claimer.as_ref(),
            ],
            &PROGRAM_ID,
        );

        let user_claim_data = ClaimStatus {
            bump: [user_claim_account_bump],
        };

        let mut user_claim_account = AccountSharedData::new(0, ClaimStatus::LEN, &system_program);
        user_claim_account.set_data_from_slice(unsafe { to_bytes(&user_claim_data) });

        let ix_data = ClaimAirdropInstructionData {
            amount: airdrop_recipients[leaf_index].1,
            leaf_index: leaf_index as u64,
            proof_len: proof.len() as u8,
            bump: user_claim_account_bump,
        };
        let mut data = vec![1];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        // add proof to data
        for proof_element in &proof {
            data.extend_from_slice(proof_element);
        }

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(airdrop_address, false),
                AccountMeta::new(claimer, true),
                AccountMeta::new(user_claim_address, false),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let result: mollusk_svm::result::InstructionResult = mollusk
            .process_and_validate_instruction(
                &instruction,
                &[
                    (airdrop_address, airdrop_account.into()),
                    (claimer, claimer_account.into()),
                    (user_claim_address, user_claim_account.into()),
                    (system_program, system_account.into()),
                ],
                &[
                    Check::err(ProgramError::Custom(2)), // already_claimed
                    Check::account(&airdrop_address).owner(&PROGRAM_ID).build(),
                    Check::account(&airdrop_address)
                        .lamports(amount + lamport_for_rent)
                        .build(),
                ],
            );
        assert!(result.program_result == ProgramResult::Failure(ProgramError::Custom(2)));
    }

    #[test]
    fn update_merkle_tree_success() {
        let mollusk = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let maker_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let old_airdrop_recipients = vec![
            (Pubkey::new_unique(), 100_000_000u64),
            (Pubkey::new_unique(), 200_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
            (Pubkey::new_unique(), 75_000_000u64),
            (Pubkey::new_unique(), 125_000_000u64),
        ];
        let old_merkle_root = create_merkle_root(&old_airdrop_recipients);
        let old_amount: u64 = old_airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let (airdrop_address, airdrop_account_bump) =
            Pubkey::find_program_address(&[AirdropState::SEED], &PROGRAM_ID);

        let airdrop_account_data = AirdropState {
            authority: maker.to_bytes(),
            merkle_root: old_merkle_root,
            airdrop_amount: old_amount.to_le_bytes(),
            amount_claimed: 0u64.to_le_bytes(),
            bump: [airdrop_account_bump],
        };
        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(AirdropState::LEN);

        let mut airdrop_account = AccountSharedData::new(
            lamport_for_rent + old_amount,
            AirdropState::LEN,
            &PROGRAM_ID,
        );

        airdrop_account
            .set_data_from_slice(unsafe { to_bytes::<AirdropState>(&airdrop_account_data) });

        let new_airdrop_recipients = vec![
            (Pubkey::new_unique(), 300_000_000u64),
            (Pubkey::new_unique(), 20_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
            (Pubkey::new_unique(), 720_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
        ];
        let new_merkle_root = create_merkle_root(&new_airdrop_recipients);
        let new_amount: u64 = new_airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let ix_data = UpdateMerkleRootInstructionData {
            new_merkle_root,
            additional_amount: new_amount - old_amount,
        };

        let mut data = vec![2];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(airdrop_address, false),
                AccountMeta::new(maker, true),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let result: mollusk_svm::result::InstructionResult = mollusk
            .process_and_validate_instruction(
                &instruction,
                &[
                    (airdrop_address, airdrop_account.into()),
                    (maker, maker_account.into()),
                    (system_program, system_account.into()),
                ],
                &[
                    Check::success(),
                    Check::account(&airdrop_address).owner(&PROGRAM_ID).build(),
                ],
            );
        assert!(result.program_result == ProgramResult::Success);
    }

    #[test]
    fn update_merkle_tree_failure_with_unauthorized() {
        let mollusk = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let _maker_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let old_airdrop_recipients = vec![
            (Pubkey::new_unique(), 100_000_000u64),
            (Pubkey::new_unique(), 200_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
            (Pubkey::new_unique(), 75_000_000u64),
            (Pubkey::new_unique(), 125_000_000u64),
        ];
        let old_merkle_root = create_merkle_root(&old_airdrop_recipients);
        let old_amount: u64 = old_airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let (airdrop_address, airdrop_account_bump) =
            Pubkey::find_program_address(&[AirdropState::SEED], &PROGRAM_ID);

        let airdrop_account_data = AirdropState {
            authority: maker.to_bytes(),
            merkle_root: old_merkle_root,
            airdrop_amount: old_amount.to_le_bytes(),
            amount_claimed: 0u64.to_le_bytes(),
            bump: [airdrop_account_bump],
        };
        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(AirdropState::LEN);

        let mut airdrop_account = AccountSharedData::new(
            lamport_for_rent + old_amount,
            AirdropState::LEN,
            &PROGRAM_ID,
        );

        airdrop_account
            .set_data_from_slice(unsafe { to_bytes::<AirdropState>(&airdrop_account_data) });

        let new_airdrop_recipients = vec![
            (Pubkey::new_unique(), 300_000_000u64),
            (Pubkey::new_unique(), 20_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
            (Pubkey::new_unique(), 720_000_000u64),
            (Pubkey::new_unique(), 150_000_000u64),
        ];
        let new_merkle_root = create_merkle_root(&new_airdrop_recipients);
        let new_amount: u64 = new_airdrop_recipients.iter().map(|(_, amt)| amt).sum();

        let ix_data = UpdateMerkleRootInstructionData {
            new_merkle_root,
            additional_amount: new_amount - old_amount,
        };

        let mut data = vec![2];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        let fake_maker = Pubkey::new_from_array([0x05; 32]);
        let fake_maker_account = Account::new(1 * LAMPORTS_PER_SOL, 0, &system_program);

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(airdrop_address, false),
                AccountMeta::new(fake_maker, true),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let result: mollusk_svm::result::InstructionResult = mollusk
            .process_and_validate_instruction(
                &instruction,
                &[
                    (airdrop_address, airdrop_account.into()),
                    (fake_maker, fake_maker_account.into()),
                    (system_program, system_account.into()),
                ],
                &[
                    Check::err(ProgramError::Custom(1)), // unauthorized
                    Check::account(&airdrop_address).owner(&PROGRAM_ID).build(),
                    Check::account(&airdrop_address)
                        .data(unsafe { to_bytes::<AirdropState>(&airdrop_account_data) })
                        .build(),
                ],
            );
        assert!(result.program_result == ProgramResult::Failure(ProgramError::Custom(1)));
    }

    #[test]
    fn test_create_merkle_root_and_proof() {
        use pinocchio_airdrop_distributor::utils::{create_airdrop_leaf, verify_merkle_proof};

        let airdrop_recipients = vec![
            (Pubkey::new_unique(), 1000u64),
            (Pubkey::new_unique(), 2000u64),
            (Pubkey::new_unique(), 1500u64),
            (Pubkey::new_unique(), 3000u64),
        ];

        let merkle_root = create_merkle_root(&airdrop_recipients);
        println!(
            "Generated merkle root (first 8 bytes): {:?}",
            &merkle_root[..8]
        );

        let empty_root = create_merkle_root(&[]);
        assert_eq!(empty_root, [0u8; 32]);

        let single_recipient = vec![(Pubkey::new_unique(), 5000u64)];
        let single_root = create_merkle_root(&single_recipient);

        let expected_leaf =
            create_airdrop_leaf(&single_recipient[0].0.to_bytes(), single_recipient[0].1, 0);
        assert_eq!(single_root, expected_leaf);

        for (index, (pubkey, amount)) in airdrop_recipients.iter().enumerate() {
            let proof = create_merkle_proof(&airdrop_recipients, index);
            let leaf = create_airdrop_leaf(&pubkey.to_bytes(), *amount, 0);

            // Verify proof
            let is_valid = verify_merkle_proof(&leaf, &proof, index as u64, &merkle_root);
            assert!(is_valid, "Proof verification failed for index {}", index);
        }

        println!("✅ Merkle root creation and proof verification successful");
    }
}
