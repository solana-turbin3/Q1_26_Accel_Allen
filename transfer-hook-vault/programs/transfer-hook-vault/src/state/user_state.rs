use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserState {
    pub user: Pubkey,
    pub amount_deposited: u64,
    pub bump: u8,
}
