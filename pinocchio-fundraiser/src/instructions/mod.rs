pub mod initialize;
pub mod create_contributor;
pub mod contribute;
pub mod checker;
pub mod refund;

pub use initialize::*;
pub use create_contributor::*;
pub use contribute::*;
pub use checker::*;
pub use refund::*;

use pinocchio::error::ProgramError;

pub enum FundraiserInstruction {
    Initialize = 0,
    CreateContributor = 1,
    Contribute = 2,
    Checker = 3,
    Refund = 4,
}

impl TryFrom<&u8> for FundraiserInstruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundraiserInstruction::Initialize),
            1 => Ok(FundraiserInstruction::CreateContributor),
            2 => Ok(FundraiserInstruction::Contribute),
            3 => Ok(FundraiserInstruction::Checker),
            4 => Ok(FundraiserInstruction::Refund),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
