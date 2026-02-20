use anchor_lang::{prelude::msg, AccountDeserialize, InstructionData, Space, ToAccountMetas};
use solana_keypair::Keypair;
use solana_instruction::Instruction;
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_signer::Signer;
use solana_transaction::Transaction;

use super::helpers::*;

fn do_apply_merkle_root_update(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
) -> Result<(), String> {
    let (vault_config_pda, _) = get_vault_config_pda();

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: convert_account_metas(
            crate::accounts::ApplyMerkleRootUpdate {
                vault_config: to_anchor_pubkey(&vault_config_pda),
            }
            .to_account_metas(None),
        ),
        data: crate::instruction::ApplyMerkleRootUpdate {}.data(),
    };

    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[payer], msg, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| {
        e.meta.logs.join("\n")
    })
}

fn set_pending_merkle_root(
    svm: &mut litesvm::LiteSVM,
    _admin: &Keypair,
    new_root: [u8; 32],
) {
    // Directly modify VaultConfig account to set pending_merkle_root.
    // This simulates schedule_merkle_root_update without needing the tuktuk program.
    let (vault_config_pda, _) = get_vault_config_pda();
    let config_acct = svm.get_account(&vault_config_pda).unwrap();
    let mut config = crate::state::VaultConfig::try_deserialize(
        &mut config_acct.data.as_ref(),
    ).unwrap();

    config.pending_merkle_root = new_root;

    // Serialize back and set the account
    let mut data = vec![0u8; 8 + crate::state::VaultConfig::INIT_SPACE as usize];
    let mut cursor = std::io::Cursor::new(&mut data[..]);
    // Write discriminator (first 8 bytes from existing account)
    std::io::Write::write_all(&mut cursor, &config_acct.data[..8]).unwrap();
    anchor_lang::AnchorSerialize::serialize(&config, &mut cursor).unwrap();

    let mut modified_acct = config_acct.clone();
    modified_acct.data = data;
    svm.set_account(vault_config_pda, modified_acct).unwrap();
}

#[test]
fn test_apply_merkle_root_update() {
    let (mut svm, admin) = setup();
    let mint = Keypair::new();
    let original_root = [1u8; 32];

    do_initialize(&mut svm, &admin, &mint, original_root, 1_000_000);
    do_init_extra_account_meta(&mut svm, &admin, &mint.pubkey());

    // Set a pending root directly (simulating schedule_merkle_root_update)
    let new_root = [42u8; 32];
    set_pending_merkle_root(&mut svm, &admin, new_root);

    // Verify pending root was set
    let (vault_config_pda, _) = get_vault_config_pda();
    let config_acct = svm.get_account(&vault_config_pda).unwrap();
    let config = crate::state::VaultConfig::try_deserialize(
        &mut config_acct.data.as_ref(),
    ).unwrap();
    assert_eq!(config.pending_merkle_root, new_root);
    assert_eq!(config.merkle_root, original_root);

    // Apply the pending update (anyone can call this)
    let result = do_apply_merkle_root_update(&mut svm, &admin);
    assert!(result.is_ok(), "apply_merkle_root_update failed: {:?}", result.err());

    // Verify root was applied and pending was cleared
    let config_acct = svm.get_account(&vault_config_pda).unwrap();
    let config = crate::state::VaultConfig::try_deserialize(
        &mut config_acct.data.as_ref(),
    ).unwrap();
    assert_eq!(config.merkle_root, new_root);
    assert_eq!(config.pending_merkle_root, [0u8; 32]);

    msg!("test_apply_merkle_root_update passed");
}

#[test]
fn test_apply_fails_when_no_pending_root() {
    let (mut svm, admin) = setup();
    let mint = Keypair::new();

    do_initialize(&mut svm, &admin, &mint, [1u8; 32], 0);

    // Try to apply with no pending root — should fail
    let result = do_apply_merkle_root_update(&mut svm, &admin);
    assert!(result.is_err(), "Should fail when no pending root");
    let err = result.unwrap_err();
    assert!(err.contains("NoPendingMerkleRoot") || err.contains("0x1775"),
        "Expected NoPendingMerkleRoot error, got: {}", err);

    msg!("test_apply_fails_when_no_pending_root passed");
}

#[test]
fn test_apply_called_by_non_admin() {
    let (mut svm, admin) = setup();
    let mint = Keypair::new();

    do_initialize(&mut svm, &admin, &mint, [1u8; 32], 0);

    // Set a pending root
    let new_root = [99u8; 32];
    set_pending_merkle_root(&mut svm, &admin, new_root);

    // A non-admin (cranker) can also call apply
    let cranker = Keypair::new();
    svm.airdrop(&cranker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

    let result = do_apply_merkle_root_update(&mut svm, &cranker);
    assert!(result.is_ok(), "Non-admin should be able to apply: {:?}", result.err());

    // Verify it was applied
    let (vault_config_pda, _) = get_vault_config_pda();
    let config_acct = svm.get_account(&vault_config_pda).unwrap();
    let config = crate::state::VaultConfig::try_deserialize(
        &mut config_acct.data.as_ref(),
    ).unwrap();
    assert_eq!(config.merkle_root, new_root);
    assert_eq!(config.pending_merkle_root, [0u8; 32]);

    msg!("test_apply_called_by_non_admin passed");
}
