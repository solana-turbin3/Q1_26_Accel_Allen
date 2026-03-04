use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};

use crate::states::Fundraiser;

/// Check if fundraiser target is met, transfer vault to maker, close fundraiser
/// Accounts: [maker(s), fundraiser, vault, maker_ata, token_program]
/// Data: (none)
pub fn process_checker(
    accounts: &[AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        maker,
        fundraiser,
        vault,
        maker_ata,
        _token_program @ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load fundraiser state
    let fund_state = Fundraiser::from_account_info(fundraiser)?;

    // Verify maker
    if fund_state.maker() != *maker.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    let amount_to_raise = fund_state.amount_to_raise();
    let bump = fund_state.bump;

    // Read vault balance (offset 64 in SPL Token account layout)
    let vault_amount = {
        let vault_data = vault.try_borrow()?;
        if vault_data.len() < 72 {
            return Err(ProgramError::InvalidAccountData);
        }
        unsafe { *(vault_data.as_ptr().add(64) as *const u64) }
    };

    // Check target met
    if vault_amount < amount_to_raise {
        return Err(ProgramError::InvalidArgument);
    }

    // Build fundraiser PDA signer
    let bump_bytes = [bump];
    let signer_seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seed);

    // Transfer all vault tokens to maker
    pinocchio_token::instructions::Transfer {
        from: vault,
        to: maker_ata,
        authority: fundraiser,
        amount: vault_amount,
    }.invoke_signed(&[signer])?;

    // Close fundraiser account — return lamports to maker
    let fundraiser_lamports = fundraiser.lamports();
    maker.set_lamports(maker.lamports() + fundraiser_lamports);
    fundraiser.set_lamports(0);
    unsafe {
        let mut data = fundraiser.try_borrow_mut()?;
        core::ptr::write_bytes(data.as_mut_ptr(), 0, data.len());
    }

    Ok(())
}
