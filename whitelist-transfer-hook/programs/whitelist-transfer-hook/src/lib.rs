#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;

mod errors;
mod instructions;
mod states;

use instructions::*;

use spl_discriminator::SplDiscriminate;
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::{
    ExecuteInstruction,
};

declare_id!("95tH1b6HTvJFQMw8V8jCzxhDfW8AYZoxvqZTN7CRNEvJ");

#[program]
pub mod whitelist_transfer_hook {
    use super::*;

    /// Initialize the global config with the caller as admin.
    pub fn initialize_config(ctx: Context<InitializeConfig>) -> Result<()> {
        ctx.accounts.initialize_config(&ctx.bumps)
    }

    /// Add a user to the whitelist (admin only).
    /// Creates a PDA at seeds ["whitelist", address].
    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>, address: Pubkey) -> Result<()> {
        ctx.accounts.add_to_whitelist(address, &ctx.bumps)
    }

    /// Remove a user from the whitelist (admin only).
    /// Closes the PDA and refunds rent to admin.
    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>, address: Pubkey) -> Result<()> {
        ctx.accounts.remove_from_whitelist(address)
    }

    /// Create a Token-2022 mint with the TransferHook extension pointing to this program.
    pub fn create_mint(ctx: Context<CreateMint>, decimals: u8) -> Result<()> {
        ctx.accounts.create_mint(decimals)
    }

/// Initialize the ExtraAccountMetaList for a mint.
    /// This registers the extra accounts (whitelist PDA) needed by the transfer hook.
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        msg!("Initializing ExtraAccountMetaList with {} extra account(s)", extra_account_metas.len());

        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas,
        ).unwrap();

        Ok(())
    }

    /// Transfer hook — invoked automatically by Token-2022 during transfers.
    /// Checks that the sender is whitelisted via their PDA.
    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        ctx.accounts.transfer_hook(amount)
    }
}