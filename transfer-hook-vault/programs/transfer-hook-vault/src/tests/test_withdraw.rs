use anchor_lang::{prelude::msg, AccountDeserialize};
use solana_signer::Signer;

use super::helpers::*;

#[test]
fn test_withdraw_whitelisted() {
    let (mut svm, admin) = setup();
    let (mint, tree, users) = full_setup(&mut svm, &admin);

    let proof = tree.proof(0);
    do_create_user_state(&mut svm, &users[0], proof).unwrap();
    do_deposit(&mut svm, &users[0], &mint, 500).unwrap();

    do_withdraw(&mut svm, &users[0], &mint, 200).unwrap();

    let (vault_config_pda, _) = get_vault_config_pda();
    let vault = get_vault_ata(&vault_config_pda, &mint);
    assert_eq!(get_token_balance(&svm, &vault), 1_000_000 + 500 - 200);

    let user_ata = get_user_ata(&users[0].pubkey(), &mint);
    assert_eq!(get_token_balance(&svm, &user_ata), 10_000 - 500 + 200);

    let (user_state_pda, _) = get_user_state_pda(&users[0].pubkey());
    let acct = svm.get_account(&user_state_pda).unwrap();
    let user_state = crate::state::UserState::try_deserialize(
        &mut acct.data.as_ref(),
    ).unwrap();
    assert_eq!(user_state.amount_deposited, 300);

    msg!("test_withdraw_whitelisted passed");
}
