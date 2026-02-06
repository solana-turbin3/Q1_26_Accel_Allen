#[cfg(test)]
mod tests {

    use {
        anchor_lang::{
            prelude::msg,
            solana_program::program_pack::Pack,
            AccountDeserialize,
            InstructionData,
            ToAccountMetas
        }, anchor_spl::{
            associated_token::{
                self,
                spl_associated_token_account
            },
            token::spl_token
        },
        litesvm::LiteSVM,
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID,
            CreateAssociatedTokenAccount,
            CreateMint, MintTo
        },
        solana_rpc_client::rpc_client::RpcClient,
        solana_account::Account,
        solana_clock::Clock,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        solana_address::Address,
        std::{
            path::PathBuf,
            str::FromStr
        }
    };

    // 5 days in seconds (must match program constant)
    const FIVE_DAYS_SECONDS: i64 = 5 * 24 * 60 * 60;

    static PROGRAM_ID: Pubkey = crate::ID;

    // Setup function to initialize LiteSVM and create a payer keypair
    // Also loads an account from devnet into the LiteSVM environment (for testing purposes)
    fn setup() -> (LiteSVM, Keypair) {
        // Initialize LiteSVM and payer
        let mut program = LiteSVM::new();
        let payer = Keypair::new();
    
        // Airdrop some SOL to the payer keypair
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");
    
        // Load program SO file
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/anchor_escrow.so");
    
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
    
        program.add_program(PROGRAM_ID, &program_data);

        // Example on how to Load an account from devnet
        // LiteSVM does not have access to real Solana network data since it does not have network access,
        // so we use an RPC client to fetch account data from devnet
        let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        let account_address = Address::from_str("DRYvf71cbF2s5wgaJQvAGkghMkRcp5arvsK2w97vXhi2").unwrap();
        let fetched_account = rpc_client
            .get_account(&account_address)
            .expect("Failed to fetch account from devnet");

        // Set the fetched account in the LiteSVM environment
        // This allows us to simulate interactions with this account during testing
        program.set_account(payer.pubkey(), Account { 
            lamports: fetched_account.lamports, 
            data: fetched_account.data, 
            owner: Pubkey::from(fetched_account.owner.to_bytes()), 
            executable: fetched_account.executable, 
            rent_epoch: fetched_account.rent_epoch 
        }).unwrap();

        msg!("Lamports of fetched account: {}", fetched_account.lamports);
    
        // Return the LiteSVM instance and payer keypair
        (program, payer)
    }

    #[test]
    fn test_make() {

        // Setup the test environment by initializing LiteSVM and creating a payer keypair
        let (mut program, payer) = setup();

        // Get the maker's public key from the payer keypair
        let maker = payer.pubkey();
        
        // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
        // This done using litesvm-token's CreateMint utility which creates the mint in the LiteSVM environment
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        // This is done using litesvm-token's CreateAssociatedTokenAccount utility
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: asspciated_token_program,
                token_program: token_program,
                system_program: system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 10, seed: 123u64, receive: 10 }.data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = program.send_transaction(transaction).unwrap();

        // Log transaction details
        msg!("\n\nMake transaction sucessfull");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify the vault account and escrow account data after the "Make" instruction
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 10);
        assert_eq!(vault_data.owner, escrow);
        assert_eq!(vault_data.mint, mint_a);

        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data = crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        assert_eq!(escrow_data.seed, 123u64);
        assert_eq!(escrow_data.maker, maker);
        assert_eq!(escrow_data.mint_a, mint_a);
        assert_eq!(escrow_data.mint_b, mint_b);
        assert_eq!(escrow_data.receive, 10);

    }

    #[test]
    fn test_take() {
        // Setup the test environment
        let (mut program, payer) = setup();

        // Create maker and taker keypairs
        let maker = payer.pubkey();
        let taker_keypair = Keypair::new();
        let taker = taker_keypair.pubkey();

        // Airdrop SOL to taker for transaction fees
        program
            .airdrop(&taker, 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to taker");

        // Create two mints (Mint A and Mint B)
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create maker's ATA for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the escrow PDA
        let seed = 456u64;
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
            &PROGRAM_ID
        ).0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the vault ATA
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault: {}\n", vault);

        // Program IDs
        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint tokens to maker's ATA A
        let deposit_amount = 100u64;
        let receive_amount = 50u64;
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000)
            .send()
            .unwrap();

        // Execute the "Make" instruction first
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {
                deposit: deposit_amount,
                seed,
                receive: receive_amount
            }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&payer], message, recent_blockhash);
        program.send_transaction(transaction).unwrap();
        msg!("Make transaction successful\n");

        // Verify vault has the deposited tokens
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, deposit_amount);

        // Create taker's ATA for Mint B and mint tokens
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker_keypair, &mint_b)
            .owner(&taker)
            .send()
            .unwrap();
        msg!("Taker ATA B: {}\n", taker_ata_b);

        // Mint tokens to taker's ATA B (enough to pay the receive amount)
        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000)
            .send()
            .unwrap();

        // Derive taker's ATA for Mint A (will be init_if_needed)
        let taker_ata_a = associated_token::get_associated_token_address(&taker, &mint_a);
        msg!("Taker ATA A: {}\n", taker_ata_a);

        // Derive maker's ATA for Mint B (will be init_if_needed)
        let maker_ata_b = associated_token::get_associated_token_address(&maker, &mint_b);
        msg!("Maker ATA B: {}\n", maker_ata_b);

        // Warp time forward by 5 days to allow take
        let mut clock = program.get_sysvar::<Clock>();
        clock.unix_timestamp += FIVE_DAYS_SECONDS;
        program.set_sysvar::<Clock>(&clock);
        msg!("Warped clock forward 5 days to timestamp: {}", clock.unix_timestamp);

        // Execute the "Take" instruction
        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker,
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&taker_keypair], message, recent_blockhash);
        let tx = program.send_transaction(transaction).unwrap();

        msg!("\n\nTake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify taker received tokens from vault (Mint A)
        let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
        let taker_ata_a_data = spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
        assert_eq!(taker_ata_a_data.amount, deposit_amount);
        assert_eq!(taker_ata_a_data.owner, taker);
        assert_eq!(taker_ata_a_data.mint, mint_a);

        // Verify maker received tokens from taker (Mint B)
        let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
        let maker_ata_b_data = spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
        assert_eq!(maker_ata_b_data.amount, receive_amount);
        assert_eq!(maker_ata_b_data.owner, maker);
        assert_eq!(maker_ata_b_data.mint, mint_b);

        // Verify taker's balance decreased
        let taker_ata_b_account = program.get_account(&taker_ata_b).unwrap();
        let taker_ata_b_data = spl_token::state::Account::unpack(&taker_ata_b_account.data).unwrap();
        assert_eq!(taker_ata_b_data.amount, 1000 - receive_amount);

        // Verify vault is closed (account either doesn't exist or has 0 lamports)
        match program.get_account(&vault) {
            Some(acc) => assert_eq!(acc.lamports, 0, "Vault should be closed"),
            None => {} // Account doesn't exist, which is expected
        }

        // Verify escrow is closed
        match program.get_account(&escrow) {
            Some(acc) => assert_eq!(acc.lamports, 0, "Escrow should be closed"),
            None => {} // Account doesn't exist, which is expected
        }
    }

    #[test]
    fn test_take_before_5_days_fails() {
        // Setup the test environment
        let (mut program, payer) = setup();

        let maker = payer.pubkey();
        let taker_keypair = Keypair::new();
        let taker = taker_keypair.pubkey();

        program
            .airdrop(&taker, 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to taker");

        // Create mints
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        // Create maker's ATA for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker)
            .send()
            .unwrap();

        // Derive PDAs
        let seed = 111u64;
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
            &PROGRAM_ID
        ).0;
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint tokens to maker
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000)
            .send()
            .unwrap();

        // Execute "Make" instruction
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 100,
                seed,
                receive: 50
            }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&payer], message, recent_blockhash);
        program.send_transaction(transaction).unwrap();
        msg!("Make transaction successful");

        // Create taker's ATA and mint tokens
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker_keypair, &mint_b)
            .owner(&taker)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000)
            .send()
            .unwrap();

        let taker_ata_a = associated_token::get_associated_token_address(&taker, &mint_a);
        let maker_ata_b = associated_token::get_associated_token_address(&maker, &mint_b);

        // DO NOT warp time - try to take immediately (should fail)
        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker,
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&taker_keypair], message, recent_blockhash);

        // This should fail because 5 days haven't passed
        let result = program.send_transaction(transaction);
        assert!(result.is_err(), "Take should fail before 5 days");

        // Verify the error contains our custom error message
        let err = result.unwrap_err();
        let logs = err.meta.logs.join("\n");
        assert!(
            logs.contains("Escrow is still locked") || logs.contains("EscrowStillLocked"),
            "Expected 'EscrowStillLocked' error, got: {}",
            logs
        );
        msg!("Take correctly failed before 5 days elapsed");

        // Verify escrow still exists (wasn't closed)
        assert!(program.get_account(&escrow).is_some(), "Escrow should still exist");
    }

    #[test]
    fn test_refund() {
        // Setup the test environment
        let (mut program, payer) = setup();

        let maker = payer.pubkey();

        // Create two mints
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create maker's ATA for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the escrow PDA
        let seed = 789u64;
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
            &PROGRAM_ID
        ).0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the vault ATA
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault: {}\n", vault);

        // Program IDs
        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint tokens to maker's ATA A
        let deposit_amount = 200u64;
        let initial_balance = 1000u64;
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, initial_balance)
            .send()
            .unwrap();

        // Execute the "Make" instruction
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {
                deposit: deposit_amount,
                seed,
                receive: 100
            }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&payer], message, recent_blockhash);
        program.send_transaction(transaction).unwrap();
        msg!("Make transaction successful\n");

        // Verify vault has the deposited tokens
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, deposit_amount);

        // Verify maker's balance decreased
        let maker_ata_a_account = program.get_account(&maker_ata_a).unwrap();
        let maker_ata_a_data = spl_token::state::Account::unpack(&maker_ata_a_account.data).unwrap();
        assert_eq!(maker_ata_a_data.amount, initial_balance - deposit_amount);

        // Execute the "Refund" instruction
        let refund_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Refund {
                maker,
                mint_a,
                maker_ata_a,
                escrow,
                vault,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Refund {}.data(),
        };

        let message = Message::new(&[refund_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&payer], message, recent_blockhash);
        let tx = program.send_transaction(transaction).unwrap();

        msg!("\n\nRefund transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify maker recovered their tokens
        let maker_ata_a_account = program.get_account(&maker_ata_a).unwrap();
        let maker_ata_a_data = spl_token::state::Account::unpack(&maker_ata_a_account.data).unwrap();
        assert_eq!(maker_ata_a_data.amount, initial_balance);
        assert_eq!(maker_ata_a_data.owner, maker);
        assert_eq!(maker_ata_a_data.mint, mint_a);

        // Verify vault is closed (account either doesn't exist or has 0 lamports)
        match program.get_account(&vault) {
            Some(acc) => assert_eq!(acc.lamports, 0, "Vault should be closed"),
            None => {} // Account doesn't exist, which is expected
        }

        // Verify escrow is closed
        match program.get_account(&escrow) {
            Some(acc) => assert_eq!(acc.lamports, 0, "Escrow should be closed"),
            None => {} // Account doesn't exist, which is expected
        }
    }

}