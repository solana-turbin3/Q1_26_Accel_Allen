#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{spl_token::{self}, CreateAssociatedTokenAccount, CreateMint, MintTo};

    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    const PROGRAM_ID: &str = "4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT";
    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
    const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn setup() -> (LiteSVM, Keypair) {

        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        svm
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        // Load program SO file — try deploy dir first, then sbpf build dir
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let so_path = base.join("target/deploy/escrow.so");

        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        svm.add_program(program_id(), &program_data).expect("Failed to add program");

        (svm, payer)

    }

    const AMOUNT_TO_RECEIVE: u64 = 100_000_000; // 100 tokens (6 decimals)
    const AMOUNT_TO_GIVE: u64 = 500_000_000;    // 500 tokens (6 decimals)

    /// Helper: runs Make instruction, returns state needed by Take/Cancel tests.
    fn do_make(
        svm: &mut LiteSVM,
        payer: &Keypair,
    ) -> (Pubkey, Pubkey, Pubkey, Pubkey, u8, Pubkey) {
        let program_id = program_id();

        let mint_a = CreateMint::new(svm, payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(svm, payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(svm, payer, &mint_a)
            .owner(&payer.pubkey())
            .send()
            .unwrap();

        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), payer.pubkey().as_ref()],
            &program_id,
        );

        let vault = spl_associated_token_account::get_associated_token_address(
            &escrow.0,
            &mint_a,
        );

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        // Mint 1000 tokens of mint_a to maker
        MintTo::new(svm, payer, &mint_a, &maker_ata_a, 1_000_000_000)
            .send()
            .unwrap();

        let bump: u8 = escrow.1;

        let make_data = [
            vec![0u8], // Discriminator: Make
            bump.to_le_bytes().to_vec(),
            AMOUNT_TO_RECEIVE.to_le_bytes().to_vec(),
            AMOUNT_TO_GIVE.to_le_bytes().to_vec(),
        ]
        .concat();

        let make_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
            ],
            data: make_data,
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[payer], message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();
        println!("Make tx CUs: {}", tx.compute_units_consumed);

        // Return: (mint_a, mint_b, escrow_pda, maker_ata_a, bump, vault)
        (mint_a, mint_b, escrow.0, maker_ata_a, bump, vault)
    }

    #[test]
    pub fn test_make_instruction() {
        let (mut svm, payer) = setup();

        let program_id = program_id();

        assert_eq!(program_id.to_string(), PROGRAM_ID);

        let mint_a = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        println!("Mint A: {}", mint_a);

        let mint_b = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        println!("Mint B: {}", mint_b);

        // Create the maker's associated token account for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
            .owner(&payer.pubkey()).send().unwrap();
        println!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), payer.pubkey().as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );
        println!("Escrow PDA: {}\n", escrow.0);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = spl_associated_token_account::get_associated_token_address(
            &escrow.0,  // owner will be the escrow PDA
            &mint_a     // mint
        );
        println!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(&mut svm, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        let amount_to_receive: u64 = 100000000; // 100 tokens with 6 decimal places
        let amount_to_give: u64 = 500000000;    // 500 tokens with 6 decimal places
        let bump: u8 = escrow.1;

        println!("Bump: {}", bump);

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_data = [
            vec![0u8],              // Discriminator for "Make" instruction
            bump.to_le_bytes().to_vec(),
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ].concat();
        let make_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
            ],
            data: make_data,
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = svm.send_transaction(transaction).unwrap();

        // Log transaction details
        println!("\n\nMake transaction sucessfull");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
    }

    #[test]
    pub fn test_take_instruction() {
        let (mut svm, maker) = setup();
        let (mint_a, mint_b, escrow_pda, _maker_ata_a, _bump, vault) =
            do_make(&mut svm, &maker);

        let program_id = program_id();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();

        // Create taker
        let taker = Keypair::new();
        svm.airdrop(&taker.pubkey(), 5 * LAMPORTS_PER_SOL).unwrap();

        // Create taker's ATA for mint_a (to receive mint_a from vault)
        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Create taker's ATA for mint_b and fund it
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Mint mint_b tokens to taker (need at least AMOUNT_TO_RECEIVE)
        MintTo::new(&mut svm, &maker, &mint_b, &taker_ata_b, AMOUNT_TO_RECEIVE)
            .send()
            .unwrap();

        // Create maker's ATA for mint_b (to receive mint_b from taker)
        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        // Build Take instruction (discriminator = 1)
        let take_data = vec![1u8]; // Discriminator: Take
        let take_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),    // taker (signer)
                AccountMeta::new(maker.pubkey(), false),   // maker
                AccountMeta::new(mint_a, false),           // mint_a
                AccountMeta::new(mint_b, false),           // mint_b
                AccountMeta::new(escrow_pda, false),       // escrow
                AccountMeta::new(taker_ata_a, false),      // taker_ata_a
                AccountMeta::new(taker_ata_b, false),      // taker_ata_b
                AccountMeta::new(maker_ata_b, false),      // maker_ata_b
                AccountMeta::new(vault, false),            // vault
                AccountMeta::new(system_program, false),   // system_program
                AccountMeta::new(token_program, false),    // token_program
                AccountMeta::new(associated_token_program, false), // associated_token_program
            ],
            data: take_data,
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&taker], message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();

        println!("\n\nTake transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
    }

    #[test]
    pub fn test_cancel_instruction() {
        let (mut svm, maker) = setup();
        let (mint_a, _mint_b, escrow_pda, maker_ata_a, _bump, vault) =
            do_make(&mut svm, &maker);

        let program_id = program_id();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        // Build Cancel instruction (discriminator = 2)
        let cancel_data = vec![2u8]; // Discriminator: Cancel
        let cancel_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),  // maker (signer)
                AccountMeta::new(mint_a, false),         // mint_a
                AccountMeta::new(escrow_pda, false),     // escrow
                AccountMeta::new(maker_ata_a, false),    // maker_ata
                AccountMeta::new(vault, false),          // vault
                AccountMeta::new(token_program, false),  // token_program
                AccountMeta::new(system_program, false), // system_program
            ],
            data: cancel_data,
        };

        let message = Message::new(&[cancel_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();

        println!("\n\nCancel transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
    }

    #[test]
    pub fn test_make_v2_instruction() {
        let (mut svm, payer) = setup();
        let program_id = program_id();

        let mint_a = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
            .owner(&payer.pubkey())
            .send()
            .unwrap();

        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), payer.pubkey().as_ref()],
            &program_id,
        );

        let vault = spl_associated_token_account::get_associated_token_address(
            &escrow.0,
            &mint_a,
        );

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        MintTo::new(&mut svm, &payer, &mint_a, &maker_ata_a, 1_000_000_000)
            .send()
            .unwrap();

        let bump: u8 = escrow.1;

        // Build MakeV2 instruction (discriminator = 3), same data layout
        let make_v2_data = [
            vec![3u8], // Discriminator: MakeV2
            bump.to_le_bytes().to_vec(),
            AMOUNT_TO_RECEIVE.to_le_bytes().to_vec(),
            AMOUNT_TO_GIVE.to_le_bytes().to_vec(),
        ]
        .concat();

        let make_v2_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
            ],
            data: make_v2_data,
        };

        let message = Message::new(&[make_v2_ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&payer], message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();

        println!("\n\nMakeV2 transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
    }
}
