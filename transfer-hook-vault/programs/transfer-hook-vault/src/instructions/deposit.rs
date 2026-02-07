use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{VaultConfig, UserState};

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub user: Signer<'info>,

    #[account(
        seeds = [b"vault_config"],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [b"user_state", user.key().as_ref()],
        bump = user_state.bump,
        constraint = user_state.user == user.key() @ VaultError::NotWhitelisted,
    )]
    pub user_state: Account<'info, UserState>,
}

impl<'info> Deposit<'info> {
    pub fn handler(&mut self, amount: u64) -> Result<()> {
        // Update deposited amount (client must pair with transfer_checked in same tx)
        self.user_state.amount_deposited = self.user_state.amount_deposited
            .checked_add(amount)
            .unwrap();

        msg!("Recorded deposit of {} tokens for user {}", amount, self.user.key());
        Ok(())
    }
}
