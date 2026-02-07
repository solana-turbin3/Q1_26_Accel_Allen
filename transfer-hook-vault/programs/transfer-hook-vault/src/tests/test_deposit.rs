use anchor_lang::{prelude::msg, AccountDeserialize};
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

use super::helpers::*;

#[test]
fn test_deposit_whitelisted() {
    let (mut svm, admin) = setup();
    let (mint, tree, users) = full_setup(&mut svm, &admin);

    let proof = tree.proof(0);
    do_create_user_state(&mut svm, &users[0], proof).unwrap();

    let deposit_amount = 500u64;
    do_deposit(&mut svm, &users[0], &mint, deposit_amount).unwrap();

    let (vault_config_pda, _) = get_vault_config_pda();
    let vault = get_vault_ata(&vault_config_pda, &mint);
    assert_eq!(get_token_balance(&svm, &vault), 1_000_000 + deposit_amount);

    let user_ata = get_user_ata(&users[0].pubkey(), &mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 10_000 - deposit_amount);

    let (user_state_pda, _) = get_user_state_pda(&users[0].pubkey());
    let acct = svm.get_account(&user_state_pda).unwrap();
    let user_state = crate::state::UserState::try_deserialize(
        &mut acct.data.as_ref(),
    ).unwrap();
    assert_eq!(user_state.amount_deposited, deposit_amount);

    msg!("test_deposit_whitelisted passed");
}

#[test]
fn test_deposit_without_user_state_fails() {
    let (mut svm, admin) = setup();
    let (mint, _tree, users) = full_setup(&mut svm, &admin);

    // Try transfer_checked directly — hook should reject because no UserState PDA
    let (vault_config_pda, _) = get_vault_config_pda();
    let vault = get_vault_ata(&vault_config_pda, &mint);
    let user_ata = get_user_ata(&users[0].pubkey(), &mint);

    let transfer_ix = build_transfer_checked_ix(
        &user_ata, &mint, &vault, &users[0].pubkey(), 100,
    );

    let msg = Message::new(&[transfer_ix], Some(&users[0].pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[&users[0]], msg, blockhash);
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Transfer without user state should fail");

    msg!("test_deposit_without_user_state_fails passed");
}
