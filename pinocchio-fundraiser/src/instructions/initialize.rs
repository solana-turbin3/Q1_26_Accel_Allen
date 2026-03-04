use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::states::Fundraiser;
use crate::MIN_AMOUNT_TO_RAISE;

/// Initialize a new fundraiser
/// Accounts: [maker(s,m), fundraiser, mint_to_raise, system_program]
/// Data: [amount_to_raise: u64, duration: u8, bump: u8]
pub fn process_initialize(
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    let [
        maker,
        fundraiser,
        mint_to_raise,
        _system_program @ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Parse instruction data
    if data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount_to_raise = unsafe { *(data.as_ptr() as *const u64) };
    let duration = data[8];
    let bump = data[9];

    // Read mint decimals (offset 44 in SPL Token mint layout)
    let mint_data = mint_to_raise.try_borrow()?;
    if mint_data.len() < 45 {
        return Err(ProgramError::InvalidAccountData);
    }
    let decimals = mint_data[44];

    // Validate amount > MIN_AMOUNT_TO_RAISE^decimals
    let min_amount = MIN_AMOUNT_TO_RAISE.pow(decimals as u32);
    if amount_to_raise <= min_amount {
        return Err(ProgramError::InvalidArgument);
    }

    // Verify fundraiser PDA
    let seed = [b"fundraiser".as_ref(), maker.address().as_ref(), &[bump]];
    let fundraiser_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(fundraiser_pda, *fundraiser.address().as_array());

    // Build signer seeds
    let bump_bytes = [bump];
    let signer_seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seed);

    // Create fundraiser account
    CreateAccount {
        from: maker,
        to: fundraiser,
        lamports: Rent::get()?.try_minimum_balance(Fundraiser::LEN)?,
        space: Fundraiser::LEN as u64,
        owner: &crate::ID,
    }.invoke_signed(&[signer])?;

    // Initialize state
    let state = Fundraiser::from_account_info(fundraiser)?;
    state.set_maker(maker.address());
    state.set_mint_to_raise(mint_to_raise.address());
    state.set_amount_to_raise(amount_to_raise);
    state.set_current_amount(0);
    state.set_time_started(Clock::get()?.unix_timestamp);
    state.duration = duration;
    state.bump = bump;

    Ok(())
}
