use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub merkle_root: [u8; 32],
    pub pending_merkle_root: [u8; 32],
    pub bump: u8,
}
