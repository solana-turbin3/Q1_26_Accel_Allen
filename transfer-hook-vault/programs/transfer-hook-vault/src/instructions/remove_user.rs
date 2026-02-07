use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{VaultConfig, UserState};

#[derive(Accounts)]
#[instruction(user_to_remove: Pubkey)]
pub struct RemoveUser<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"vault_config"],
        bump = vault_config.bump,
        constraint = vault_config.admin == admin.key() @ VaultError::Unauthorized,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        close = admin,
        seeds = [b"user_state", user_to_remove.as_ref()],
        bump = user_state.bump,
        constraint = user_state.user == user_to_remove,
    )]
    pub user_state: Account<'info, UserState>,

    pub system_program: Program<'info, System>,
}

impl<'info> RemoveUser<'info> {
    pub fn handler(&mut self, user_to_remove: Pubkey) -> Result<()> {
        msg!("User removed: {}", user_to_remove);
        Ok(())
    }
}
