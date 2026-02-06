use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct WhitelistEntry {
    pub address: Pubkey,
    pub bump: u8,
}