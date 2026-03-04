#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{
        spl_token::{self},
        CreateAssociatedTokenAccount, CreateMint, MintTo,
    };
    use solana_clock::Clock;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let so_path = base.join("target/deploy/pinocchio_fundraiser.so");
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
        svm.add_program(program_id(), &program_data)
            .expect("Failed to add program");

        (svm, payer)
    }

    /// Helper: initialize a fundraiser and return (mint, fundraiser_pda, fundraiser_bump, vault)
    fn do_initialize(
        svm: &mut LiteSVM,
        maker: &Keypair,
        amount_to_raise: u64,
        duration: u8,
    ) -> (Pubkey, Pubkey, u8, Pubkey) {
        let program_id = program_id();
        let system_program = solana_sdk_ids::system_program::ID;

        let mint = CreateMint::new(svm, maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let (fundraiser_pda, bump) = Pubkey::find_program_address(
            &[b"fundraiser", maker.pubkey().as_ref()],
            &program_id,
        );

        let vault = spl_associated_token_account::get_associated_token_address(
            &fundraiser_pda,
            &mint,
        );

        // Build initialize ix: disc=0, amount:u64, duration:u8, bump:u8
        let mut ix_data = vec![0u8]; // discriminator
        ix_data.extend_from_slice(&amount_to_raise.to_le_bytes());
        ix_data.push(duration);
        ix_data.push(bump);

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(system_program, false),
            ],
            data: ix_data,
        };

        let message = Message::new(&[ix], Some(&maker.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[maker], message, blockhash);
        let result = svm.send_transaction(tx).unwrap();
        println!("Initialize CUs: {}", result.compute_units_consumed);

        (mint, fundraiser_pda, bump, vault)
    }

    /// Helper: create the vault ATA for the fundraiser PDA
    fn create_vault(
        svm: &mut LiteSVM,
        payer: &Keypair,
        fundraiser_pda: &Pubkey,
        mint: &Pubkey,
    ) -> Pubkey {
        CreateAssociatedTokenAccount::new(svm, payer, mint)
            .owner(fundraiser_pda)
            .send()
            .unwrap()
    }

    /// Helper: create contributor state PDA
    fn do_create_contributor(
        svm: &mut LiteSVM,
        contributor: &Keypair,
        fundraiser_pda: &Pubkey,
    ) -> (Pubkey, u8) {
        let program_id = program_id();
        let system_program = solana_sdk_ids::system_program::ID;

        let (contributor_state_pda, bump) = Pubkey::find_program_address(
            &[
                b"contributor",
                fundraiser_pda.as_ref(),
                contributor.pubkey().as_ref(),
            ],
            &program_id,
        );

        let ix_data = vec![1u8, bump]; // disc=1, bump

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new_readonly(*fundraiser_pda, false),
                AccountMeta::new(contributor_state_pda, false),
                AccountMeta::new_readonly(system_program, false),
            ],
            data: ix_data,
        };

        let message = Message::new(&[ix], Some(&contributor.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[contributor], message, blockhash);
        let result = svm.send_transaction(tx).unwrap();
        println!("CreateContributor CUs: {}", result.compute_units_consumed);

        (contributor_state_pda, bump)
    }

    /// Helper: contribute tokens
    fn do_contribute(
        svm: &mut LiteSVM,
        contributor: &Keypair,
        fundraiser_pda: &Pubkey,
        vault: &Pubkey,
        contributor_ata: &Pubkey,
        contributor_state_pda: &Pubkey,
        amount: u64,
    ) {
        let program_id = program_id();
        let token_program = TOKEN_PROGRAM_ID;

        let mut ix_data = vec![2u8]; // disc=2
        ix_data.extend_from_slice(&amount.to_le_bytes());

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new(*fundraiser_pda, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*contributor_ata, false),
                AccountMeta::new(*contributor_state_pda, false),
                AccountMeta::new_readonly(token_program, false),
            ],
            data: ix_data,
        };

        let message = Message::new(&[ix], Some(&contributor.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[contributor], message, blockhash);
        let result = svm.send_transaction(tx).unwrap();
        println!("Contribute CUs: {}", result.compute_units_consumed);
    }

    #[test]
    fn test_initialize() {
        let (mut svm, maker) = setup();

        // 10_000 tokens with 6 decimals = 10_000_000_000
        // MIN_AMOUNT_TO_RAISE^6 = 3^6 = 729, so 10B > 729 ✓
        let (mint, fundraiser_pda, _bump, _vault) =
            do_initialize(&mut svm, &maker, 10_000_000_000, 7);

        // Verify fundraiser account exists and is owned by our program
        let account = svm.get_account(&fundraiser_pda).unwrap();
        assert_eq!(account.owner, program_id());
        assert_eq!(account.data.len(), 96); // Fundraiser::LEN
        println!("Initialize test passed");
    }

    #[test]
    fn test_create_contributor() {
        let (mut svm, maker) = setup();
        let (_mint, fundraiser_pda, _bump, _vault) =
            do_initialize(&mut svm, &maker, 10_000_000_000, 7);

        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 5 * LAMPORTS_PER_SOL)
            .unwrap();

        let (contributor_state_pda, _) =
            do_create_contributor(&mut svm, &contributor, &fundraiser_pda);

        let account = svm.get_account(&contributor_state_pda).unwrap();
        assert_eq!(account.owner, program_id());
        assert_eq!(account.data.len(), 48); // Contributor::LEN
        println!("CreateContributor test passed");
    }

    #[test]
    fn test_contribute() {
        let (mut svm, maker) = setup();
        let amount_to_raise = 10_000_000_000u64; // 10k tokens
        let (mint, fundraiser_pda, _bump, vault) =
            do_initialize(&mut svm, &maker, amount_to_raise, 7);

        // Create vault ATA
        let vault = create_vault(&mut svm, &maker, &fundraiser_pda, &mint);

        // Set up contributor
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 5 * LAMPORTS_PER_SOL)
            .unwrap();

        let (contributor_state_pda, _) =
            do_create_contributor(&mut svm, &contributor, &fundraiser_pda);

        // Create contributor ATA and mint tokens
        let contributor_ata =
            CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
                .owner(&contributor.pubkey())
                .send()
                .unwrap();

        // Mint 2000 tokens to contributor (max contribution = 10% of 10k = 1k tokens)
        MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 2_000_000_000)
            .send()
            .unwrap();

        // Contribute 500 tokens (well within 10% cap of 1000 tokens)
        let contribute_amount = 500_000_000u64;
        do_contribute(
            &mut svm,
            &contributor,
            &fundraiser_pda,
            &vault,
            &contributor_ata,
            &contributor_state_pda,
            contribute_amount,
        );

        // Verify vault received tokens
        let vault_account = svm.get_account(&vault).unwrap();
        let vault_balance = u64::from_le_bytes(vault_account.data[64..72].try_into().unwrap());
        assert_eq!(vault_balance, contribute_amount);
        println!("Contribute test passed, vault balance: {}", vault_balance);
    }

    #[test]
    fn test_checker() {
        let (mut svm, maker) = setup();
        // Set target low enough that we can meet it with 10% contributions
        // Target = 1000 tokens, max per contributor = 100 tokens
        let amount_to_raise = 1_000_000_000u64; // 1000 tokens
        let (mint, fundraiser_pda, _bump, _) =
            do_initialize(&mut svm, &maker, amount_to_raise, 30);

        let vault = create_vault(&mut svm, &maker, &fundraiser_pda, &mint);

        // We need 10 contributors each giving 100 tokens to meet the 1000 target
        // (max contribution = 10% = 100 tokens per contributor)
        let max_contribution = 100_000_000u64; // 100 tokens
        for i in 0..10 {
            let contributor = Keypair::new();
            svm.airdrop(&contributor.pubkey(), 5 * LAMPORTS_PER_SOL)
                .unwrap();

            let (contributor_state_pda, _) =
                do_create_contributor(&mut svm, &contributor, &fundraiser_pda);

            let contributor_ata =
                CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
                    .owner(&contributor.pubkey())
                    .send()
                    .unwrap();

            MintTo::new(&mut svm, &maker, &mint, &contributor_ata, max_contribution)
                .send()
                .unwrap();

            do_contribute(
                &mut svm,
                &contributor,
                &fundraiser_pda,
                &vault,
                &contributor_ata,
                &contributor_state_pda,
                max_contribution,
            );
            println!("Contributor {} done", i);
        }

        // Create maker ATA to receive funds
        let maker_ata = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        // Call checker (disc=3)
        let program_id = program_id();
        let token_program = TOKEN_PROGRAM_ID;

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(maker_ata, false),
                AccountMeta::new_readonly(token_program, false),
            ],
            data: vec![3u8],
        };

        let message = Message::new(&[ix], Some(&maker.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[&maker], message, blockhash);
        let result = svm.send_transaction(tx).unwrap();
        println!("Checker CUs: {}", result.compute_units_consumed);

        // Verify maker received tokens
        let maker_account = svm.get_account(&maker_ata).unwrap();
        let maker_balance =
            u64::from_le_bytes(maker_account.data[64..72].try_into().unwrap());
        assert_eq!(maker_balance, amount_to_raise);

        // Verify fundraiser account closed
        assert!(svm.get_account(&fundraiser_pda).is_none());
        println!("Checker test passed, maker balance: {}", maker_balance);
    }

    #[test]
    fn test_refund() {
        let (mut svm, maker) = setup();
        let amount_to_raise = 1_000_000_000u64; // 1000 tokens
        let duration = 1u8; // 1 day

        let (mint, fundraiser_pda, _bump, _) =
            do_initialize(&mut svm, &maker, amount_to_raise, duration);

        let vault = create_vault(&mut svm, &maker, &fundraiser_pda, &mint);

        // Set up contributor
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 5 * LAMPORTS_PER_SOL)
            .unwrap();

        let (contributor_state_pda, _) =
            do_create_contributor(&mut svm, &contributor, &fundraiser_pda);

        let contributor_ata =
            CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
                .owner(&contributor.pubkey())
                .send()
                .unwrap();

        let contribute_amount = 50_000_000u64; // 50 tokens (within 10% = 100)
        MintTo::new(&mut svm, &maker, &mint, &contributor_ata, contribute_amount)
            .send()
            .unwrap();

        do_contribute(
            &mut svm,
            &contributor,
            &fundraiser_pda,
            &vault,
            &contributor_ata,
            &contributor_state_pda,
            contribute_amount,
        );

        // Verify contributor ATA is now empty
        let ata_account = svm.get_account(&contributor_ata).unwrap();
        let ata_balance = u64::from_le_bytes(ata_account.data[64..72].try_into().unwrap());
        assert_eq!(ata_balance, 0);

        // Warp time forward past duration (2 days)
        let mut clock: Clock = svm.get_sysvar();
        clock.unix_timestamp += 2 * 86400; // 2 days
        svm.set_sysvar(&clock);

        // Call refund (disc=4)
        let program_id = program_id();
        let token_program = TOKEN_PROGRAM_ID;

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new_readonly(maker.pubkey(), false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new(contributor_state_pda, false),
                AccountMeta::new_readonly(token_program, false),
            ],
            data: vec![4u8],
        };

        let message = Message::new(&[ix], Some(&contributor.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[&contributor], message, blockhash);
        let result = svm.send_transaction(tx).unwrap();
        println!("Refund CUs: {}", result.compute_units_consumed);

        // Verify contributor got tokens back
        let ata_account = svm.get_account(&contributor_ata).unwrap();
        let refunded_balance =
            u64::from_le_bytes(ata_account.data[64..72].try_into().unwrap());
        assert_eq!(refunded_balance, contribute_amount);

        // Verify contributor state closed
        assert!(svm.get_account(&contributor_state_pda).is_none());
        println!("Refund test passed, refunded: {}", refunded_balance);
    }
}
