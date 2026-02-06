use anchor_lang::prelude::*;

use crate::errors::WhitelistError;
use crate::states::{Config, WhitelistEntry};

#[derive(Accounts)]
#[instruction(address: Pubkey)]
pub struct AddToWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
        constraint = config.admin == admin.key() @ WhitelistError::Unauthorized,
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = admin,
        space = 8 + WhitelistEntry::INIT_SPACE,
        seeds = [b"whitelist", address.as_ref()],
        bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, address: Pubkey, bumps: &AddToWhitelistBumps) -> Result<()> {
        self.whitelist_entry.set_inner(WhitelistEntry {
            address,
            bump: bumps.whitelist_entry,
        });

        msg!("Address added to whitelist: {}", address);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(address: Pubkey)]
pub struct RemoveFromWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
        constraint = config.admin == admin.key() @ WhitelistError::Unauthorized,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        close = admin,
        seeds = [b"whitelist", address.as_ref()],
        bump = whitelist_entry.bump,
        constraint = whitelist_entry.address == address,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self, address: Pubkey) -> Result<()> {
        msg!("Address removed from whitelist: {}", address);
        Ok(())
    }
}