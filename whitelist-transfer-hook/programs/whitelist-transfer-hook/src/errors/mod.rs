use anchor_lang::prelude::*;

#[error_code]
pub enum WhitelistError {
    #[msg("The address is not whitelisted")]
    NotWhitelisted,

    #[msg("This instruction must be called from a transfer hook")]
    NotTransferring,

    #[msg("Unauthorized: Only the admin can perform this action")]
    Unauthorized,

    #[msg("The address is already whitelisted")]
    AlreadyWhitelisted,
}