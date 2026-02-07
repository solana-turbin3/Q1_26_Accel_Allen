use anchor_lang::{prelude::msg, AccountDeserialize};
use solana_keypair::Keypair;
use solana_signer::Signer;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions, transfer_hook::TransferHook};
use spl_token_2022::state::Mint as MintState;
use solana_pubkey::Pubkey;

use super::helpers::*;

#[test]
fn test_initialize() {
    let (mut svm, admin) = setup();
    let mint = Keypair::new();
    let root = [1u8; 32];

    let vault_config_pda = do_initialize(&mut svm, &admin, &mint, root, 1_000_000);

    // Verify VaultConfig
    let config_acct = svm.get_account(&vault_config_pda).unwrap();
    let config = crate::state::VaultConfig::try_deserialize(
        &mut config_acct.data.as_ref(),
    ).unwrap();
    assert_eq!(config.admin, to_anchor_pubkey(&admin.pubkey()));
    assert_eq!(config.mint, to_anchor_pubkey(&mint.pubkey()));
    assert_eq!(config.merkle_root, root);

    // Verify mint has TransferHook extension
    let mint_acct = svm.get_account(&mint.pubkey()).unwrap();
    let mint_state = StateWithExtensions::<MintState>::unpack(&mint_acct.data).unwrap();
    let hook_ext = mint_state.get_extension::<TransferHook>().unwrap();
    let hook_program = Pubkey::from(hook_ext.program_id.0.to_bytes());
    assert_eq!(hook_program, PROGRAM_ID);

    // Verify vault has initial supply
    let vault = get_vault_ata(&vault_config_pda, &mint.pubkey());
    assert_eq!(get_token_balance(&svm, &vault), 1_000_000);

    msg!("test_initialize passed");
}

#[test]
fn test_init_extra_account_meta() {
    let (mut svm, admin) = setup();
    let mint = Keypair::new();
    do_initialize(&mut svm, &admin, &mint, [0u8; 32], 0);
    do_init_extra_account_meta(&mut svm, &admin, &mint.pubkey());

    let (extra_meta_pda, _) = get_extra_account_meta_list_pda(&mint.pubkey());
    let acct = svm.get_account(&extra_meta_pda);
    assert!(acct.is_some(), "ExtraAccountMetaList should exist");
    assert!(acct.unwrap().data.len() > 0, "ExtraAccountMetaList should have data");

    msg!("test_init_extra_account_meta passed");
}
