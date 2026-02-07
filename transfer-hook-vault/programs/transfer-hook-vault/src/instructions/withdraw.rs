use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use anchor_spl::token_interface::spl_token_2022;

use crate::errors::VaultError;
use crate::state::{VaultConfig, UserState};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault_config"],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [b"approval", user.key().as_ref()],
        bump = approval.bump,
        constraint = approval.user == user.key() @ VaultError::NotWhitelisted,
    )]
    pub approval: Account<'info, UserState>,

    #[account(
        mut,
        address = vault_config.vault,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Withdraw<'info> {
    pub fn handler(&mut self, amount: u64) -> Result<()> {
        // Check withdraw doesn't exceed deposited
        require!(
            amount <= self.approval.amount_deposited,
            VaultError::WithdrawExceedsDeposited
        );

        // Approve user as delegate on vault ATA so they can call transfer_checked
        let ix = spl_token_2022::instruction::approve(
            &self.token_program.key(),
            &self.vault.key(),
            &self.user.key(),
            &self.vault_config.key(),
            &[],
            amount,
        )?;

        let signer_seeds: &[&[u8]] = &[
            b"vault_config",
            &[self.vault_config.bump],
        ];

        invoke_signed(
            &ix,
            &[
                self.vault.to_account_info(),
                self.user.to_account_info(),
                self.vault_config.to_account_info(),
            ],
            &[signer_seeds],
        )?;

        // Update deposited amount
        self.approval.amount_deposited = self.approval.amount_deposited
            .checked_sub(amount)
            .ok_or(VaultError::WithdrawExceedsDeposited)?;

        msg!("Approved withdrawal of {} tokens for user {}", amount, self.user.key());
        Ok(())
    }
}
