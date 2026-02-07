use anchor_lang::{prelude::msg, AccountDeserialize, InstructionData, ToAccountMetas};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID;
use solana_signer::Signer;
use solana_transaction::Transaction;

use super::helpers::*;

#[test]
fn test_update_merkle_root() {
    let (mut svm, admin) = setup();
    let mint = Keypair::new();
    do_initialize(&mut svm, &admin, &mint, [0u8; 32], 0);

    let new_root = [42u8; 32];
    let (vault_config_pda, _) = get_vault_config_pda();

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: convert_account_metas(
            crate::accounts::UpdateMerkleRoot {
                admin: to_anchor_pubkey(&admin.pubkey()),
                vault_config: to_anchor_pubkey(&vault_config_pda),
            }
            .to_account_metas(None),
        ),
        data: crate::instruction::UpdateMerkleRoot { new_root }.data(),
    };

    let msg = Message::new(&[ix], Some(&admin.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[&admin], msg, blockhash);
    svm.send_transaction(tx).unwrap();

    let config_acct = svm.get_account(&vault_config_pda).unwrap();
    let config = crate::state::VaultConfig::try_deserialize(
        &mut config_acct.data.as_ref(),
    ).unwrap();
    assert_eq!(config.merkle_root, new_root);

    msg!("test_update_merkle_root passed");
}

#[test]
fn test_create_user_state() {
    let (mut svm, admin) = setup();
    let (_mint, tree, users) = full_setup(&mut svm, &admin);

    let proof = tree.proof(0);
    do_create_user_state(&mut svm, &users[0], proof).unwrap();

    let (user_state_pda, _) = get_user_state_pda(&users[0].pubkey());
    let acct = svm.get_account(&user_state_pda).unwrap();
    let user_state = crate::state::UserState::try_deserialize(
        &mut acct.data.as_ref(),
    ).unwrap();
    assert_eq!(user_state.user, to_anchor_pubkey(&users[0].pubkey()));
    assert_eq!(user_state.amount_deposited, 0);

    msg!("test_create_user_state passed");
}

#[test]
fn test_create_user_state_invalid_proof() {
    let (mut svm, admin) = setup();
    let (_mint, tree, users) = full_setup(&mut svm, &admin);

    let wrong_proof = tree.proof(1);
    let result = do_create_user_state(&mut svm, &users[0], wrong_proof);
    assert!(result.is_err(), "Create user state with invalid proof should fail");
    assert!(
        result.unwrap_err().contains("InvalidMerkleProof"),
        "Error should mention InvalidMerkleProof"
    );

    msg!("test_create_user_state_invalid_proof passed");
}

#[test]
fn test_remove_user() {
    let (mut svm, admin) = setup();
    let (_mint, tree, users) = full_setup(&mut svm, &admin);

    let proof = tree.proof(0);
    do_create_user_state(&mut svm, &users[0], proof).unwrap();

    let (user_state_pda, _) = get_user_state_pda(&users[0].pubkey());
    assert!(svm.get_account(&user_state_pda).is_some());

    let (vault_config_pda, _) = get_vault_config_pda();
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: convert_account_metas(
            crate::accounts::RemoveUser {
                admin: to_anchor_pubkey(&admin.pubkey()),
                vault_config: to_anchor_pubkey(&vault_config_pda),
                user_state: to_anchor_pubkey(&user_state_pda),
                system_program: to_anchor_pubkey(&SYSTEM_PROGRAM_ID),
            }
            .to_account_metas(None),
        ),
        data: crate::instruction::RemoveUser {
            user_to_remove: to_anchor_pubkey(&users[0].pubkey()),
        }
        .data(),
    };

    let msg = Message::new(&[ix], Some(&admin.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[&admin], msg, blockhash);
    svm.send_transaction(tx).unwrap();

    let acct = svm.get_account(&user_state_pda);
    assert!(
        acct.is_none() || acct.unwrap().lamports == 0,
        "UserState should be closed"
    );

    msg!("test_remove_user passed");
}
