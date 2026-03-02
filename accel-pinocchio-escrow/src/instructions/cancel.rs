use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};

use crate::state::Escrow;

pub fn process_cancel_instruction(
    accounts: &[AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        maker,
        mint_a,
        escrow_account,
        maker_ata,
        vault,
        _token_program,
        _system_program @ ..
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify maker is signer
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load escrow state
    let escrow_state = Escrow::from_account_info(escrow_account)?;

    // Verify maker matches signer
    if escrow_state.maker() != *maker.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify mint_a matches
    if escrow_state.mint_a() != *mint_a.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    let amount_to_give = escrow_state.amount_to_give();
    let bump = escrow_state.bump;

    // Build escrow PDA signer seeds
    let bump_bytes = [bump];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&seed);

    // 1) Transfer mint_a: vault -> maker_ata (escrow PDA signs)
    pinocchio_token::instructions::Transfer {
        from: vault,
        to: maker_ata,
        authority: escrow_account,
        amount: amount_to_give,
    }
    .invoke_signed(&[signer.clone()])?;

    // 2) Close vault ATA -> maker (escrow PDA signs)
    pinocchio_token::instructions::CloseAccount {
        account: vault,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[signer])?;

    // 3) Close escrow account -> return lamports to maker
    {
        let escrow_lamports = escrow_account.lamports();
        maker.set_lamports(maker.lamports() + escrow_lamports);
        escrow_account.set_lamports(0);

        // Zero out escrow data
        unsafe {
            let mut data = escrow_account.try_borrow_mut()?;
            core::ptr::write_bytes(data.as_mut_ptr(), 0, data.len());
        }
    }

    Ok(())
}
