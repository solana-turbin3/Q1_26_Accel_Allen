use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::states::Contributor;

/// Create a contributor state account (separate from contribute to avoid init_if_needed)
/// Accounts: [contributor(s,m), fundraiser, contributor_state, system_program]
/// Data: [bump: u8]
pub fn process_create_contributor(
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    let [
        contributor,
        fundraiser,
        contributor_state,
        _system_program @ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let bump = data[0];

    // Verify contributor PDA
    let seed = [
        b"contributor".as_ref(),
        fundraiser.address().as_ref(),
        contributor.address().as_ref(),
        &[bump],
    ];
    let contributor_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(contributor_pda, *contributor_state.address().as_array());

    // Build signer seeds
    let bump_bytes = [bump];
    let signer_seed = [
        Seed::from(b"contributor"),
        Seed::from(fundraiser.address().as_array()),
        Seed::from(contributor.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seed);

    // Create contributor state account
    CreateAccount {
        from: contributor,
        to: contributor_state,
        lamports: Rent::get()?.try_minimum_balance(Contributor::LEN)?,
        space: Contributor::LEN as u64,
        owner: &crate::ID,
    }.invoke_signed(&[signer])?;

    // Initialize state
    let state = Contributor::from_account_info(contributor_state)?;
    state.set_contributor(contributor.address());
    state.set_amount(0);
    state.bump = bump;

    Ok(())
}
