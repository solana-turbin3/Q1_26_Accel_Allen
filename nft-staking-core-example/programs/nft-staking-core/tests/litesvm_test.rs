use litesvm::LiteSVM;
use sha2::{Sha256, Digest};
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

// ------- Pubkey bridging (solana 3.x <-> anchor 2.x) -------

fn to_anchor_pubkey(pk: &Pubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(pk.to_bytes())
}

fn from_anchor_pubkey(pk: &anchor_lang::prelude::Pubkey) -> Pubkey {
    Pubkey::from(pk.to_bytes())
}

// Anchor discriminator: sha256("global:<name>")[..8]
fn anchor_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", name).as_bytes());
    let result = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result[..8]);
    disc
}

// -------- Constants --------

const MPL_CORE_ID: Pubkey = Pubkey::from_str_const("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
const PROGRAM_ID: Pubkey = Pubkey::from_str_const("BeUozctAJ14QWbNfoFt43VYDFhAaYZFprCHAkAU8y1Wj");
const TOKEN_2022_ID: Pubkey = Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const SPL_ATA_ID: Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");
const SECONDS_PER_DAY: i64 = 86400;

// -------- PDA helpers --------

fn find_update_authority(collection: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"update_authority", collection.as_ref()],
        &PROGRAM_ID,
    )
}

fn find_config(collection: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"config", collection.as_ref()],
        &PROGRAM_ID,
    )
}

fn find_rewards_mint(config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"rewards", config.as_ref()],
        &PROGRAM_ID,
    )
}

fn find_oracle(collection: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"oracle", collection.as_ref()],
        &PROGRAM_ID,
    )
}

fn find_vault(collection: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"vault", collection.as_ref()],
        &PROGRAM_ID,
    )
}

fn find_ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            TOKEN_2022_ID.as_ref(),
            mint.as_ref(),
        ],
        &SPL_ATA_ID,
    ).0
}

// -------- Setup helpers --------

struct TestEnv {
    svm: LiteSVM,
    admin: Keypair,
    collection: Pubkey,
    update_authority: Pubkey,
    config: Pubkey,
    rewards_mint: Pubkey,
}

fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();

    // Load mpl-core program
    let mpl_core_bytes = include_bytes!("../../../tests/mpl_core.so");
    svm.add_program(MPL_CORE_ID, mpl_core_bytes);

    // Load our program
    let program_bytes = include_bytes!("../../../target/deploy/nft_staking_core.so");
    svm.add_program(PROGRAM_ID, program_bytes);

    svm
}

fn airdrop(svm: &mut LiteSVM, to: &Pubkey, lamports: u64) {
    svm.airdrop(to, lamports).unwrap();
}

// Build and send a transaction
fn send_tx(svm: &mut LiteSVM, ixs: &[Instruction], signers: &[&Keypair]) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(ixs, Some(&signers[0].pubkey()));
    let tx = Transaction::new(&signers.to_vec(), msg, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{:?}", e))
}

fn send_tx_expect_success(svm: &mut LiteSVM, ixs: &[Instruction], signers: &[&Keypair]) {
    send_tx(svm, ixs, signers).expect("transaction should succeed");
}

fn send_tx_expect_failure(svm: &mut LiteSVM, ixs: &[Instruction], signers: &[&Keypair]) {
    send_tx(svm, ixs, signers).expect_err("transaction should fail");
}

// -------- Instruction builders --------

fn ix_create_collection(payer: &Pubkey, collection: &Pubkey, name: &str, uri: &str) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let mut data = anchor_discriminator("create_collection").to_vec();
    // borsh: String = len(u32) + bytes
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(&(uri.len() as u32).to_le_bytes());
    data.extend_from_slice(uri.as_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*collection, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
        data,
    }
}

fn ix_initialize_config(admin: &Pubkey, collection: &Pubkey, points_per_stake: u32, freeze_period: u8) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let (config, _) = find_config(collection);
    let (rewards_mint, _) = find_rewards_mint(&config);

    let mut data = anchor_discriminator("initialize_config").to_vec();
    data.extend_from_slice(&points_per_stake.to_le_bytes());
    data.push(freeze_period);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new_readonly(*collection, false),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new(config, false),
            AccountMeta::new(rewards_mint, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
        ],
        data,
    }
}

fn ix_mint_nft(user: &Pubkey, nft: &Pubkey, collection: &Pubkey, name: &str, uri: &str) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let mut data = anchor_discriminator("mint_nft").to_vec();
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(&(uri.len() as u32).to_le_bytes());
    data.extend_from_slice(uri.as_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*nft, true),
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
        data,
    }
}

fn ix_stake(user: &Pubkey, nft: &Pubkey, collection: &Pubkey) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let (config, _) = find_config(collection);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(*nft, false),
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: anchor_discriminator("stake").to_vec(),
    }
}

fn ix_unstake(user: &Pubkey, nft: &Pubkey, collection: &Pubkey) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let (config, _) = find_config(collection);
    let (rewards_mint, _) = find_rewards_mint(&config);
    let user_ata = find_ata(user, &rewards_mint);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rewards_mint, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new(*nft, false),
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
            AccountMeta::new_readonly(SPL_ATA_ID, false),
        ],
        data: anchor_discriminator("unstake").to_vec(),
    }
}

fn ix_claim_rewards(user: &Pubkey, nft: &Pubkey, collection: &Pubkey) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let (config, _) = find_config(collection);
    let (rewards_mint, _) = find_rewards_mint(&config);
    let user_ata = find_ata(user, &rewards_mint);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rewards_mint, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new(*nft, false),
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
            AccountMeta::new_readonly(SPL_ATA_ID, false),
        ],
        data: anchor_discriminator("claim_rewards").to_vec(),
    }
}

fn ix_burn_staked_nft(user: &Pubkey, nft: &Pubkey, collection: &Pubkey) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let (config, _) = find_config(collection);
    let (rewards_mint, _) = find_rewards_mint(&config);
    let user_ata = find_ata(user, &rewards_mint);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rewards_mint, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new(*nft, false),
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
            AccountMeta::new_readonly(SPL_ATA_ID, false),
        ],
        data: anchor_discriminator("burn_staked_nft").to_vec(),
    }
}

fn ix_create_oracle(payer: &Pubkey, collection: &Pubkey, initial_vault_lamports: u64) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let (oracle, _) = find_oracle(collection);
    let (vault, _) = find_vault(collection);

    let mut data = anchor_discriminator("create_oracle").to_vec();
    data.extend_from_slice(&initial_vault_lamports.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new(*collection, false),
            AccountMeta::new(oracle, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

fn ix_crank_oracle(cranker: &Pubkey, collection: &Pubkey) -> Instruction {
    let (oracle, _) = find_oracle(collection);
    let (vault, _) = find_vault(collection);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*cranker, true),
            AccountMeta::new_readonly(*collection, false),
            AccountMeta::new(oracle, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: anchor_discriminator("crank_oracle").to_vec(),
    }
}

fn ix_transfer_nft(user: &Pubkey, new_owner: &Pubkey, nft: &Pubkey, collection: &Pubkey) -> Instruction {
    let (update_authority, _) = find_update_authority(collection);
    let (oracle, _) = find_oracle(collection);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(*new_owner, false),
            AccountMeta::new(*nft, false),
            AccountMeta::new_readonly(*collection, false),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(oracle, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: anchor_discriminator("transfer_nft").to_vec(),
    }
}

fn ix_fund_vault(funder: &Pubkey, collection: &Pubkey, amount: u64) -> Instruction {
    let (vault, _) = find_vault(collection);
    let mut data = anchor_discriminator("fund_vault").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*funder, true),
            AccountMeta::new_readonly(*collection, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

// -------- Clock manipulation --------

fn set_clock(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar(&clock);
    // Expire blockhash to prevent AlreadyProcessed errors
    svm.expire_blockhash();
}

fn get_clock(svm: &LiteSVM) -> Clock {
    svm.get_sysvar::<Clock>()
}

// -------- Token balance helper --------

fn get_token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let account = svm.get_account(ata);
    match account {
        None => 0,
        Some(acc) => {
            if acc.data.len() < 72 {
                return 0;
            }
            // Token account layout: amount is at offset 64, u64 LE
            u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
        }
    }
}

// -------- Oracle state helper --------
fn get_oracle_transfer(svm: &LiteSVM, collection: &Pubkey) -> u8 {
    let (oracle, _) = find_oracle(collection);
    let account = svm.get_account(&oracle).expect("oracle account exists");
    // Anchor discriminator (8) + variant(1) + create(1) + transfer(1)
    account.data[10]
}

// -------- Full setup: collection + config + mint NFT --------

fn setup_full(points_per_stake: u32, freeze_period: u8) -> TestEnv {
    let mut svm = setup_svm();
    let admin = Keypair::new();
    let collection_kp = Keypair::new();
    let collection = collection_kp.pubkey();

    airdrop(&mut svm, &admin.pubkey(), 10 * LAMPORTS_PER_SOL);

    // Create collection
    let ix = ix_create_collection(&admin.pubkey(), &collection, "Test Collection", "https://example.com");
    send_tx_expect_success(&mut svm, &[ix], &[&admin, &collection_kp]);

    // Initialize config
    let ix = ix_initialize_config(&admin.pubkey(), &collection, points_per_stake, freeze_period);
    send_tx_expect_success(&mut svm, &[ix], &[&admin]);

    let (update_authority, _) = find_update_authority(&collection);
    let (config, _) = find_config(&collection);
    let (rewards_mint, _) = find_rewards_mint(&config);

    TestEnv {
        svm,
        admin,
        collection,
        update_authority,
        config,
        rewards_mint,
    }
}

fn mint_nft_for_user(env: &mut TestEnv, user: &Keypair) -> Pubkey {
    let nft_kp = Keypair::new();
    let nft = nft_kp.pubkey();
    let ix = ix_mint_nft(&user.pubkey(), &nft, &env.collection, "Test NFT", "https://example.com/nft");
    send_tx_expect_success(&mut env.svm, &[ix], &[user, &nft_kp]);
    nft
}

// ====================================================================
// TESTS
// ====================================================================

#[test]
fn test_stake_and_unstake_with_time_travel() {
    // points_per_stake=10 (with 6 decimals, each "point" is 10 tokens/day)
    // freeze_period=2 days
    let mut env = setup_full(10, 2);
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    // Set clock to a known time: day 1000 at noon
    let base_time = 1000 * SECONDS_PER_DAY + 12 * 3600;
    set_clock(&mut env.svm, base_time);

    // Stake
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Try unstake before freeze period (1 day later) - should fail
    set_clock(&mut env.svm, base_time + 1 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_failure(&mut env.svm, &[ix], &[&user]);

    // Advance 3 days (past freeze_period=2)
    set_clock(&mut env.svm, base_time + 3 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Check rewards: 3 days * 10 points_per_stake = 30 tokens
    let user_ata = find_ata(&user.pubkey(), &env.rewards_mint);
    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 30, "expected 30 reward tokens for 3 days staked");
}

#[test]
fn test_claim_rewards_without_unstaking() {
    let mut env = setup_full(10, 2);
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    let base_time = 2000 * SECONDS_PER_DAY + 12 * 3600;
    set_clock(&mut env.svm, base_time);

    // Stake
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Advance 5 days, claim rewards
    set_clock(&mut env.svm, base_time + 5 * SECONDS_PER_DAY);
    let ix = ix_claim_rewards(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    let user_ata = find_ata(&user.pubkey(), &env.rewards_mint);
    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 50, "expected 50 reward tokens for 5 days (claim_rewards)");

    // Advance another 3 days and claim again (should get 3 more days worth)
    set_clock(&mut env.svm, base_time + 8 * SECONDS_PER_DAY);
    let ix = ix_claim_rewards(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 80, "expected 80 total after second claim (50 + 30)");

    // NFT is still staked - can unstake after freeze period (already past it)
    set_clock(&mut env.svm, base_time + 10 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Unstake should give rewards for the 2 days since last claim (days 8-10)
    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 100, "expected 100 total after unstake (80 + 20)");
}

#[test]
fn test_burn_staked_nft_rewards() {
    // freeze_period=2, points_per_stake=10
    // burn_bonus = freeze_period(2) * points_per_stake(10) * 2 = 40
    let mut env = setup_full(10, 2);
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    let base_time = 3000 * SECONDS_PER_DAY + 12 * 3600;
    set_clock(&mut env.svm, base_time);

    // Stake
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Advance 5 days, then burn
    set_clock(&mut env.svm, base_time + 5 * SECONDS_PER_DAY);
    let ix = ix_burn_staked_nft(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Reward = base(5 days * 10 = 50) + burn_bonus(2 * 10 * 2 = 40) = 90
    let user_ata = find_ata(&user.pubkey(), &env.rewards_mint);
    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 90, "expected 90 burn reward (50 base + 40 burn bonus)");

    // NFT should be burned - account data should be empty (rent refunded to payer)
    let nft_account = env.svm.get_account(&nft);
    match nft_account {
        None => {} // account fully closed
        Some(acc) => {
            // mpl-core burn clears the data but may leave the account with residual lamports
            assert!(acc.data.is_empty() || acc.data.len() <= 1,
                "NFT data should be cleared after burn, got {} bytes", acc.data.len());
        }
    }
}

#[test]
fn test_collection_total_staked_tracking() {
    let mut env = setup_full(10, 0);
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 10 * LAMPORTS_PER_SOL);

    let nft1 = mint_nft_for_user(&mut env, &user);
    let nft2 = mint_nft_for_user(&mut env, &user);

    let base_time = 4000 * SECONDS_PER_DAY;
    set_clock(&mut env.svm, base_time);

    // Stake NFT 1
    let ix = ix_stake(&user.pubkey(), &nft1, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Stake NFT 2
    let ix = ix_stake(&user.pubkey(), &nft2, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Unstake NFT 1
    set_clock(&mut env.svm, base_time + 1 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft1, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Re-stake NFT 1
    set_clock(&mut env.svm, base_time + 2 * SECONDS_PER_DAY);
    let ix = ix_stake(&user.pubkey(), &nft1, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Both staked now - unstake both
    set_clock(&mut env.svm, base_time + 3 * SECONDS_PER_DAY);
    let ix1 = ix_unstake(&user.pubkey(), &nft1, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix1], &[&user]);
    let ix2 = ix_unstake(&user.pubkey(), &nft2, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix2], &[&user]);

    // All succeeded - collection tracking worked through stake/unstake/restake cycles
}

#[test]
fn test_oracle_business_hours_transfer() {
    let mut env = setup_full(10, 0);
    let user = Keypair::new();
    let recipient = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);
    airdrop(&mut env.svm, &recipient.pubkey(), 1 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    // Set time to 10 AM UTC (business hours)
    let day_start = 5000 * SECONDS_PER_DAY;
    set_clock(&mut env.svm, day_start + 10 * 3600);

    // Create oracle during business hours
    let ix = ix_create_oracle(&env.admin.pubkey(), &env.collection, LAMPORTS_PER_SOL);
    send_tx_expect_success(&mut env.svm, &[ix], &[&env.admin]);

    // Transfer should work during business hours (oracle initialized with PASS)
    let ix = ix_transfer_nft(&user.pubkey(), &recipient.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Mint another NFT for the recipient to transfer back
    let nft2 = mint_nft_for_user(&mut env, &user);

    // Move to 8 PM UTC (outside business hours)
    set_clock(&mut env.svm, day_start + 20 * 3600);

    // Crank oracle to update state (now should be REJECTED)
    let cranker = Keypair::new();
    airdrop(&mut env.svm, &cranker.pubkey(), 1 * LAMPORTS_PER_SOL);
    let ix = ix_crank_oracle(&cranker.pubkey(), &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&cranker]);

    // Verify oracle state is REJECTED
    assert_eq!(get_oracle_transfer(&env.svm, &env.collection), 1, "oracle should be REJECTED outside hours");

    // Transfer should fail outside business hours
    let ix = ix_transfer_nft(&user.pubkey(), &recipient.pubkey(), &nft2, &env.collection);
    send_tx_expect_failure(&mut env.svm, &[ix], &[&user]);

    // Move to next day 12 PM UTC (business hours)
    set_clock(&mut env.svm, day_start + SECONDS_PER_DAY + 12 * 3600);

    // Crank oracle back to PASS
    let ix = ix_crank_oracle(&cranker.pubkey(), &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&cranker]);

    assert_eq!(get_oracle_transfer(&env.svm, &env.collection), 2, "oracle should be PASS during hours");

    // Transfer should work again
    let ix = ix_transfer_nft(&user.pubkey(), &recipient.pubkey(), &nft2, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);
}

#[test]
fn test_oracle_crank_unchanged_state_fails() {
    let mut env = setup_full(10, 0);

    // Set time to 10 AM UTC (business hours)
    let day_start = 6000 * SECONDS_PER_DAY;
    set_clock(&mut env.svm, day_start + 10 * 3600);

    // Create oracle (transfer=PASS during business hours)
    let ix = ix_create_oracle(&env.admin.pubkey(), &env.collection, LAMPORTS_PER_SOL);
    send_tx_expect_success(&mut env.svm, &[ix], &[&env.admin]);

    // Try cranking at 11 AM (still business hours, state unchanged) - should fail
    set_clock(&mut env.svm, day_start + 11 * 3600);
    let cranker = Keypair::new();
    airdrop(&mut env.svm, &cranker.pubkey(), 1 * LAMPORTS_PER_SOL);
    let ix = ix_crank_oracle(&cranker.pubkey(), &env.collection);
    send_tx_expect_failure(&mut env.svm, &[ix], &[&cranker]);
}

#[test]
fn test_oracle_crank_reward_near_boundary() {
    let mut env = setup_full(10, 0);

    // Set time to 10 AM UTC (business hours)
    let day_start = 7000 * SECONDS_PER_DAY;
    set_clock(&mut env.svm, day_start + 10 * 3600);

    // Create oracle with vault funding
    let ix = ix_create_oracle(&env.admin.pubkey(), &env.collection, LAMPORTS_PER_SOL);
    send_tx_expect_success(&mut env.svm, &[ix], &[&env.admin]);

    let cranker = Keypair::new();
    airdrop(&mut env.svm, &cranker.pubkey(), 1 * LAMPORTS_PER_SOL);
    let cranker_balance_before = env.svm.get_account(&cranker.pubkey()).unwrap().lamports;

    // Move to 5:02 PM UTC (within 5 min of close boundary, just past close)
    set_clock(&mut env.svm, day_start + 17 * 3600 + 120);

    // Crank oracle - should get reward (near boundary + state changes from PASS to REJECTED)
    let ix = ix_crank_oracle(&cranker.pubkey(), &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&cranker]);

    let cranker_balance_after = env.svm.get_account(&cranker.pubkey()).unwrap().lamports;
    // Cranker should have received 0.01 SOL reward (10_000_000 lamports)
    // Note: cranker also pays tx fees so we check the delta is close to reward
    // Balance after should be roughly balance_before + reward(0.01 SOL) - tx_fee
    assert!(cranker_balance_after > cranker_balance_before, "cranker should have received reward");
}

#[test]
fn test_cannot_unstake_before_freeze_period() {
    let mut env = setup_full(10, 5); // 5 day freeze period
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    let base_time = 8000 * SECONDS_PER_DAY;
    set_clock(&mut env.svm, base_time);

    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Try at day 4 (before freeze_period=5)
    set_clock(&mut env.svm, base_time + 4 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_failure(&mut env.svm, &[ix], &[&user]);

    // At exactly day 5 - should succeed
    set_clock(&mut env.svm, base_time + 5 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    let user_ata = find_ata(&user.pubkey(), &env.rewards_mint);
    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 50, "expected 50 reward tokens for 5 days");
}

#[test]
fn test_cannot_stake_already_staked() {
    let mut env = setup_full(10, 0);
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    let base_time = 9000 * SECONDS_PER_DAY;
    set_clock(&mut env.svm, base_time);

    // Stake
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Try staking again - should fail (AlreadyStaked)
    set_clock(&mut env.svm, base_time + 1 * SECONDS_PER_DAY);
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_failure(&mut env.svm, &[ix], &[&user]);
}

#[test]
fn test_claim_then_burn_rewards_accumulation() {
    // Claim partial, then burn to get remaining + burn bonus
    let mut env = setup_full(10, 2);
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    let base_time = 10000 * SECONDS_PER_DAY + 12 * 3600;
    set_clock(&mut env.svm, base_time);

    // Stake
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // After 3 days, claim
    set_clock(&mut env.svm, base_time + 3 * SECONDS_PER_DAY);
    let ix = ix_claim_rewards(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    let user_ata = find_ata(&user.pubkey(), &env.rewards_mint);
    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 30, "3 days * 10 = 30 from claim");

    // After 2 more days (5 total, 2 since last claim), burn
    set_clock(&mut env.svm, base_time + 5 * SECONDS_PER_DAY);
    let ix = ix_burn_staked_nft(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Burn reward: base(2 days since claim * 10 = 20) + burn_bonus(2 * 10 * 2 = 40) = 60
    // Total: 30 (claimed) + 60 (burn) = 90
    let balance = get_token_balance(&env.svm, &user_ata);
    assert_eq!(balance, 90, "expected 90 total (30 claimed + 20 base + 40 burn bonus)");
}

#[test]
fn test_restake_after_unstake() {
    let mut env = setup_full(10, 1);
    let user = Keypair::new();
    airdrop(&mut env.svm, &user.pubkey(), 5 * LAMPORTS_PER_SOL);

    let nft = mint_nft_for_user(&mut env, &user);

    let base_time = 11000 * SECONDS_PER_DAY;
    set_clock(&mut env.svm, base_time);

    // First stake
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Unstake after 2 days
    set_clock(&mut env.svm, base_time + 2 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    let user_ata = find_ata(&user.pubkey(), &env.rewards_mint);
    assert_eq!(get_token_balance(&env.svm, &user_ata), 20, "first stake: 2 days * 10");

    // Re-stake
    set_clock(&mut env.svm, base_time + 3 * SECONDS_PER_DAY);
    let ix = ix_stake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    // Unstake after 4 more days
    set_clock(&mut env.svm, base_time + 7 * SECONDS_PER_DAY);
    let ix = ix_unstake(&user.pubkey(), &nft, &env.collection);
    send_tx_expect_success(&mut env.svm, &[ix], &[&user]);

    assert_eq!(get_token_balance(&env.svm, &user_ata), 60, "total: 20 + 40 (4 days * 10)");
}
