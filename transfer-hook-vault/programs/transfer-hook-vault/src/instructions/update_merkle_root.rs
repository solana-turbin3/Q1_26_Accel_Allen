use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::VaultConfig;

#[derive(Accounts)]
pub struct UpdateMerkleRoot<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault_config"],
        bump = vault_config.bump,
        constraint = vault_config.admin == admin.key() @ VaultError::Unauthorized,
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

impl<'info> UpdateMerkleRoot<'info> {
    pub fn handler(&mut self, new_root: [u8; 32]) -> Result<()> {
        self.vault_config.merkle_root = new_root;
        msg!("Merkle root updated");
        Ok(())
    }
}
