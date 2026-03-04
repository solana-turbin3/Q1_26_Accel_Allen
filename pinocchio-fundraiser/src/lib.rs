#![allow(unexpected_cfgs)]
use pinocchio::{AccountView, entrypoint, Address, ProgramResult, address::declare_id, error::ProgramError};

use crate::instructions::FundraiserInstruction;

mod states;
mod instructions;
mod tests;

entrypoint!(process_instruction);

declare_id!("FUNDrXoH7qEm2GhQyGbg6MaiMjfXaVeRfEfjkiGBLbq6");

pub const MIN_AMOUNT_TO_RAISE: u64 = 3;
pub const SECONDS_TO_DAYS: i64 = 86400;
pub const MAX_CONTRIBUTION_PERCENTAGE: u64 = 10;
pub const PERCENTAGE_SCALER: u64 = 100;

pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data.split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match FundraiserInstruction::try_from(discriminator)? {
        FundraiserInstruction::Initialize => instructions::process_initialize(accounts, data)?,
        FundraiserInstruction::CreateContributor => instructions::process_create_contributor(accounts, data)?,
        FundraiserInstruction::Contribute => instructions::process_contribute(accounts, data)?,
        FundraiserInstruction::Checker => instructions::process_checker(accounts, data)?,
        FundraiserInstruction::Refund => instructions::process_refund(accounts, data)?,
    }
    Ok(())
}
