use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::TransferV1CpiBuilder,
    types::UpdateAuthority,
};
use crate::state::StakingOracle;
use crate::errors::StakingError;

#[derive(Accounts)]
pub struct TransferNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: New owner of the NFT
    pub new_owner: UncheckedAccount<'info>,
    /// CHECK: NFT account
    #[account(mut)]
    pub nft: UncheckedAccount<'info>,
    /// CHECK: Collection account
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"oracle", collection.key().as_ref()],
        bump = oracle.bump,
    )]
    pub oracle: Account<'info, StakingOracle>,
    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> TransferNft<'info> {
    pub fn transfer_nft(&self) -> Result<()> {
        // Verify NFT owner
        let base_asset = BaseAssetV1::try_from(&self.nft.to_account_info())?;
        require!(base_asset.owner == self.user.key(), StakingError::InvalidOwner);
        require!(base_asset.update_authority == UpdateAuthority::Collection(self.collection.key()), StakingError::InvalidAuthority);
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(base_collection.update_authority == self.update_authority.key(), StakingError::InvalidAuthority);

        // Check oracle allows transfer
        require!(self.oracle.transfer == StakingOracle::PASS, StakingError::TransferNotAllowed);

        // Transfer NFT via CPI, passing oracle as remaining account
        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.nft.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.user.to_account_info()))
            .new_owner(&self.new_owner.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .add_remaining_account(&self.oracle.to_account_info(), false, false)
            .invoke()?;

        Ok(())
    }
}
