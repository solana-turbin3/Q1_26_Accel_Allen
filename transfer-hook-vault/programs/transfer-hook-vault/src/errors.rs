use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Invalid Merkle proof")]
    InvalidMerkleProof,

    #[msg("Unauthorized: only the admin can perform this action")]
    Unauthorized,

    #[msg("Withdraw exceeds deposited amount")]
    WithdrawExceedsDeposited,

    #[msg("User is not whitelisted")]
    NotWhitelisted,

    #[msg("This instruction must be called from a transfer hook")]
    NotTransferring,
}
