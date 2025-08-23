#[cfg(test)]
mod tests_airdrop_distributor {
    use mollusk_svm::{
        result::{Check, ProgramResult},
        Mollusk,
    };
    use pinocchio_airdrop_distributor::{
        instructions::InitializeAirdropInstructionData, states::AirdropState, utils::to_bytes, *,
    };
    use solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
    };

    pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(ID);

    fn get_mollusk() -> Mollusk {
        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/pinocchio_airdrop_distributor");
        mollusk
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

        let ix_data = InitializeAirdropInstructionData {
            merkle_root: [0u8; 32],
            amount: 100u64,
            bump,
        };

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
}
