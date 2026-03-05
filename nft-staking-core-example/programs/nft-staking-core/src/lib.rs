use anchor_lang::prelude::*;

mod state;
mod instructions;
mod errors;
use instructions::*;

declare_id!("BeUozctAJ14QWbNfoFt43VYDFhAaYZFprCHAkAU8y1Wj");

#[program]
pub mod nft_staking_core {
    use super::*;

    pub fn create_collection(ctx: Context<CreateCollection>, name: String, uri: String) -> Result<()> {
        ctx.accounts.create_collection(name, uri, &ctx.bumps)
    }

    pub fn mint_nft(ctx: Context<Mint>, name: String, uri: String) -> Result<()> {
        ctx.accounts.mint_nft(name, uri, &ctx.bumps)
    }

    pub fn initialize_config(ctx: Context<InitConfig>, points_per_stake: u32, freeze_period: u8) -> Result<()> {
        ctx.accounts.init_config(points_per_stake, freeze_period, &ctx.bumps)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.stake(&ctx.bumps)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.unstake(&ctx.bumps)
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.claim_rewards(&ctx.bumps)
    }

    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        ctx.accounts.burn_staked_nft(&ctx.bumps)
    }

    pub fn create_oracle(ctx: Context<CreateOracle>, initial_vault_lamports: u64) -> Result<()> {
        ctx.accounts.create_oracle(&ctx.bumps, initial_vault_lamports)
    }

    pub fn crank_oracle(ctx: Context<CrankOracle>) -> Result<()> {
        ctx.accounts.crank_oracle(&ctx.bumps)
    }

    pub fn transfer_nft(ctx: Context<TransferNft>) -> Result<()> {
        ctx.accounts.transfer_nft()
    }

    pub fn fund_vault(ctx: Context<FundVault>, amount: u64) -> Result<()> {
        ctx.accounts.fund_vault(amount)
    }
}
