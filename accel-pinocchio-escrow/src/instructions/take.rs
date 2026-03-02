use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};

use crate::state::Escrow;

pub fn process_take_instruction(
    accounts: &[AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        taker,
        maker,
        mint_a,
        mint_b,
        escrow_account,
        taker_ata_a,
        taker_ata_b,
        maker_ata_b,
        vault,
        _system_program,
        _token_program,
        _associated_token_program @ ..
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify taker is signer
    if !taker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load escrow state
    let escrow_state = Escrow::from_account_info(escrow_account)?;

    // Verify maker matches
    if escrow_state.maker() != *maker.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify mints match
    if escrow_state.mint_a() != *mint_a.address() {
        return Err(ProgramError::InvalidAccountData);
    }
    if escrow_state.mint_b() != *mint_b.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    let amount_to_receive = escrow_state.amount_to_receive();
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

    // 1) Transfer mint_b: taker_ata_b -> maker_ata_b (taker signs)
    pinocchio_token::instructions::Transfer {
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: amount_to_receive,
    }
    .invoke()?;

    // 2) Transfer mint_a: vault -> taker_ata_a (escrow PDA signs)
    pinocchio_token::instructions::Transfer {
        from: vault,
        to: taker_ata_a,
        authority: escrow_account,
        amount: amount_to_give,
    }
    .invoke_signed(&[signer.clone()])?;

    // 3) Close vault ATA -> maker (escrow PDA signs)
    pinocchio_token::instructions::CloseAccount {
        account: vault,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[signer])?;

    // 4) Close escrow account -> return lamports to maker
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
