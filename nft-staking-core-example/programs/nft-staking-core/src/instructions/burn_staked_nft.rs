use anchor_lang::prelude::*;
use anchor_spl::{token_interface::{Mint, TokenInterface, TokenAccount, MintToChecked, mint_to_checked}, associated_token::AssociatedToken};
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{BurnV1CpiBuilder, UpdatePluginV1CpiBuilder, UpdateCollectionPluginV1CpiBuilder},
    types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginType, UpdateAuthority}
};
use crate::state::Config;
use crate::errors::StakingError;

const SECONDS_PER_DAY: i64 = 86400;
const BURN_BONUS_MULTIPLIER: u64 = 2;

#[derive(Accounts)]
pub struct BurnStakedNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [b"rewards", config.key().as_ref()],
        bump = config.rewards_bump
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = rewards_mint,
        associated_token::authority = user,
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: NFT account
    #[account(mut)]
    pub nft: UncheckedAccount<'info>,
    /// CHECK: Collection account
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> BurnStakedNft<'info> {
    pub fn burn_staked_nft(&mut self, bumps: &BurnStakedNftBumps) -> Result<()> {
        // Verify NFT owner and update authority
        let base_asset = BaseAssetV1::try_from(&self.nft.to_account_info())?;
        require!(base_asset.owner == self.user.key(), StakingError::InvalidOwner);
        require!(base_asset.update_authority == UpdateAuthority::Collection(self.collection.key()), StakingError::InvalidAuthority);
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(base_collection.update_authority == self.update_authority.key(), StakingError::InvalidAuthority);

        let collection_key = self.collection.key();
        let signer_seeds = &[
            b"update_authority",
            collection_key.as_ref(),
            &[bumps.update_authority],
        ];

        let current_timestamp = Clock::get()?.unix_timestamp;

        // Fetch and verify staked status
        let fetched_attribute_list = match fetch_plugin::<BaseAssetV1, Attributes>(&self.nft.to_account_info(), PluginType::Attributes) {
            Err(_) => return Err(StakingError::NotStaked.into()),
            Ok((_, attributes, _)) => attributes,
        };

        let mut staked_value: Option<&str> = None;
        let mut staked_at_value: Option<&str> = None;

        for attribute in &fetched_attribute_list.attribute_list {
            match attribute.key.as_str() {
                "staked" => staked_value = Some(&attribute.value),
                "staked_at" => staked_at_value = Some(&attribute.value),
                _ => {}
            }
        }

        require!(staked_value == Some("true"), StakingError::NotStaked);

        let staked_at_timestamp = staked_at_value
            .ok_or(StakingError::InvalidTimestamp)?
            .parse::<i64>()
            .map_err(|_| StakingError::InvalidTimestamp)?;

        let elapsed_seconds = current_timestamp
            .checked_sub(staked_at_timestamp)
            .ok_or(StakingError::InvalidTimestamp)?;

        let staked_time_days = elapsed_seconds
            .checked_div(SECONDS_PER_DAY)
            .ok_or(StakingError::InvalidTimestamp)?;

        // Calculate reward: normal rewards + burn bonus (2x multiplier on top)
        let base_reward = (staked_time_days as u64)
            .checked_mul(self.config.points_per_stake as u64)
            .ok_or(StakingError::Overflow)?;

        let burn_bonus = (self.config.freeze_period as u64)
            .checked_mul(self.config.points_per_stake as u64)
            .ok_or(StakingError::Overflow)?
            .checked_mul(BURN_BONUS_MULTIPLIER)
            .ok_or(StakingError::Overflow)?;

        let total_reward = base_reward
            .checked_add(burn_bonus)
            .ok_or(StakingError::Overflow)?;

        // Unfreeze the NFT first (FreezeDelegate blocks burns while frozen)
        UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.nft.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(&[signer_seeds])?;

        // Burn the NFT via CPI (using update authority as BurnDelegate)
        BurnV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.nft.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(&[signer_seeds])?;

        // Decrement collection total_staked
        self.decrement_collection_total_staked(signer_seeds)?;

        // Mint rewards
        let config_seeds = &[
            b"config",
            collection_key.as_ref(),
            &[self.config.config_bump],
        ];

        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = MintToChecked {
            mint: self.rewards_mint.to_account_info(),
            to: self.user_rewards_ata.to_account_info(),
            authority: self.config.to_account_info(),
        };
        let config_signer_seeds = &[&config_seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, config_signer_seeds);
        mint_to_checked(cpi_ctx, total_reward, self.rewards_mint.decimals)?;

        Ok(())
    }

    fn decrement_collection_total_staked(&self, signer_seeds: &[&[u8]]) -> Result<()> {
        match fetch_plugin::<BaseCollectionV1, Attributes>(&self.collection.to_account_info(), PluginType::Attributes) {
            Ok((_, attrs, _)) => {
                let mut attr_list: Vec<Attribute> = Vec::new();
                for attr in attrs.attribute_list {
                    if attr.key == "total_staked" {
                        let count: u64 = attr.value.parse().unwrap_or(0);
                        attr_list.push(Attribute {
                            key: "total_staked".to_string(),
                            value: count.saturating_sub(1).to_string(),
                        });
                    } else {
                        attr_list.push(attr);
                    }
                }
                UpdateCollectionPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
                    .collection(&self.collection.to_account_info())
                    .payer(&self.user.to_account_info())
                    .authority(Some(&self.update_authority.to_account_info()))
                    .system_program(&self.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes { attribute_list: attr_list }))
                    .invoke_signed(&[signer_seeds])?;
            }
            Err(_) => {} // No collection attributes yet, nothing to decrement
        }
        Ok(())
    }
}
