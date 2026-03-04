use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};

use crate::states::{Fundraiser, Contributor};
use crate::SECONDS_TO_DAYS;

/// Refund a contributor if fundraiser expired and target not met
/// Accounts: [contributor(s), maker, fundraiser, vault, contributor_ata, contributor_state, token_program]
/// Data: (none)
pub fn process_refund(
    accounts: &[AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        contributor,
        maker,
        fundraiser,
        vault,
        contributor_ata,
        contributor_state,
        _token_program @ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load fundraiser state
    let fund_state = Fundraiser::from_account_info(fundraiser)?;

    // Verify maker matches
    if fund_state.maker() != *maker.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check duration exceeded
    let current_time = Clock::get()?.unix_timestamp;
    let days_elapsed = ((current_time - fund_state.time_started()) / SECONDS_TO_DAYS) as u8;
    if days_elapsed <= fund_state.duration {
        return Err(ProgramError::InvalidArgument);
    }

    // Check target NOT met
    let vault_amount = {
        let vault_data = vault.try_borrow()?;
        if vault_data.len() < 72 {
            return Err(ProgramError::InvalidAccountData);
        }
        unsafe { *(vault_data.as_ptr().add(64) as *const u64) }
    };
    if vault_amount >= fund_state.amount_to_raise() {
        return Err(ProgramError::InvalidArgument);
    }

    // Load contributor state
    let contrib_state = Contributor::from_account_info(contributor_state)?;
    let refund_amount = contrib_state.amount();
    let bump = fund_state.bump;

    // Build fundraiser PDA signer
    let bump_bytes = [bump];
    let signer_seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seed);

    // Transfer tokens from vault back to contributor
    pinocchio_token::instructions::Transfer {
        from: vault,
        to: contributor_ata,
        authority: fundraiser,
        amount: refund_amount,
    }.invoke_signed(&[signer])?;

    // Update fundraiser current_amount
    let fund_state = Fundraiser::from_account_info(fundraiser)?;
    fund_state.set_current_amount(fund_state.current_amount() - refund_amount);

    // Close contributor state account — return lamports to contributor
    let contrib_lamports = contributor_state.lamports();
    contributor.set_lamports(contributor.lamports() + contrib_lamports);
    contributor_state.set_lamports(0);
    unsafe {
        let mut data = contributor_state.try_borrow_mut()?;
        core::ptr::write_bytes(data.as_mut_ptr(), 0, data.len());
    }

    Ok(())
}
