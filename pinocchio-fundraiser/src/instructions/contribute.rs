use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};

use crate::states::{Fundraiser, Contributor};
use crate::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER, SECONDS_TO_DAYS};

/// Contribute tokens to the fundraiser
/// Accounts: [contributor(s), fundraiser, vault, contributor_ata, contributor_state, token_program]
/// Data: [amount: u64]
pub fn process_contribute(
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    let [
        contributor,
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

    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = unsafe { *(data.as_ptr() as *const u64) };

    // Load fundraiser state
    let fund_state = Fundraiser::from_account_info(fundraiser)?;
    let amount_to_raise = fund_state.amount_to_raise();
    let max_contribution = (amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER;

    // Validate amount > 0
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    // Validate amount <= 10% of target
    if amount > max_contribution {
        return Err(ProgramError::InvalidArgument);
    }

    // Check fundraising duration has NOT been exceeded
    let current_time = Clock::get()?.unix_timestamp;
    let days_elapsed = ((current_time - fund_state.time_started()) / SECONDS_TO_DAYS) as u8;
    if days_elapsed > fund_state.duration {
        return Err(ProgramError::InvalidArgument);
    }

    // Load contributor state
    let contrib_state = Contributor::from_account_info(contributor_state)?;

    // Check per-contributor cap
    if contrib_state.amount() + amount > max_contribution {
        return Err(ProgramError::InvalidArgument);
    }

    // Transfer tokens from contributor_ata to vault
    pinocchio_token::instructions::Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount,
    }.invoke()?;

    // Update fundraiser current_amount
    let fund_state = Fundraiser::from_account_info(fundraiser)?;
    fund_state.set_current_amount(fund_state.current_amount() + amount);

    // Update contributor amount
    let contrib_state = Contributor::from_account_info(contributor_state)?;
    contrib_state.set_amount(contrib_state.amount() + amount);

    Ok(())
}
