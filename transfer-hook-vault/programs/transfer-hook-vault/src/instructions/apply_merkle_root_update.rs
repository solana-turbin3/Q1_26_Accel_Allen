use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::VaultConfig;

#[derive(Accounts)]
pub struct ApplyMerkleRootUpdate<'info> {
    #[account(
        mut,
        seeds = [b"vault_config"],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

impl<'info> ApplyMerkleRootUpdate<'info> {
    pub fn handler(&mut self) -> Result<()> {
        require!(
            self.vault_config.pending_merkle_root != [0u8; 32],
            VaultError::NoPendingMerkleRoot
        );

        self.vault_config.merkle_root = self.vault_config.pending_merkle_root;
        self.vault_config.pending_merkle_root = [0u8; 32];

        msg!("Pending merkle root applied");
        Ok(())
    }
}
