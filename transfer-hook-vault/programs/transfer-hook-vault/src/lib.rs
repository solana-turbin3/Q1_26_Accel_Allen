#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;

mod errors;
mod instructions;
mod state;
mod tuktuk_types;
mod tests;

use instructions::*;
use tuktuk_types::tuktuk::types::TriggerV0;

use spl_transfer_hook_interface::instruction::ExecuteInstruction;
use spl_discriminator::SplDiscriminate;

declare_id!("4Uoq2yp6eCji8xx6H7F1SgWWV732TnJhK7rjcyWMp7Fs");

#[program]
pub mod transfer_hook_vault {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        merkle_root: [u8; 32],
        initial_supply: u64,
    ) -> Result<()> {
        ctx.accounts.handler(merkle_root, initial_supply, &ctx.bumps)
    }

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        ctx.accounts.handler()
    }

    pub fn update_merkle_root(
        ctx: Context<UpdateMerkleRoot>,
        new_root: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.handler(new_root)
    }

    pub fn create_user_state(
        ctx: Context<CreateUserState>,
        proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        ctx.accounts.handler(proof, &ctx.bumps)
    }

    pub fn remove_user(
        ctx: Context<RemoveUser>,
        user_to_remove: Pubkey,
    ) -> Result<()> {
        ctx.accounts.handler(user_to_remove)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.handler(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.handler(amount)
    }

    pub fn schedule_merkle_root_update(
        ctx: Context<ScheduleMerkleRootUpdate>,
        new_root: [u8; 32],
        task_id: u16,
        trigger: TriggerV0,
    ) -> Result<()> {
        ctx.accounts.handler(new_root, task_id, trigger, &ctx.bumps)
    }

    pub fn apply_merkle_root_update(
        ctx: Context<ApplyMerkleRootUpdate>,
    ) -> Result<()> {
        ctx.accounts.handler()
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHookCtx>, amount: u64) -> Result<()> {
        ctx.accounts.handler(amount)
    }
}
