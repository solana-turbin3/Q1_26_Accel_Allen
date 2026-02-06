use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, Debug)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive: u64,
    pub bump: u8,
    pub created_at: i64,  // Unix timestamp when escrow was created
}

// 5 days in seconds
pub const ESCROW_LOCK_DURATION: i64 = 5 * 24 * 60 * 60;