use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::BaseCollectionV1,
    instructions::AddCollectionExternalPluginAdapterV1CpiBuilder,
    types::{
        ExternalPluginAdapterInitInfo, OracleInitInfo, ExternalCheckResult,
        HookableLifecycleEvent, PluginAuthority, ValidationResultsOffset,
    }
};
use crate::state::StakingOracle;
use crate::errors::StakingError;

const SECONDS_PER_DAY: i64 = 86400;
const OPEN_HOUR: i64 = 9;   // 9 AM UTC
const CLOSE_HOUR: i64 = 17; // 5 PM UTC

#[derive(Accounts)]
pub struct CreateOracle<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    /// CHECK: Collection account
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + StakingOracle::INIT_SPACE,
        seeds = [b"oracle", collection.key().as_ref()],
        bump
    )]
    pub oracle: Account<'info, StakingOracle>,
    /// CHECK: Vault PDA for crank rewards (just holds lamports)
    #[account(
        mut,
        seeds = [b"vault", collection.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>,
    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateOracle<'info> {
    pub fn create_oracle(&mut self, bumps: &CreateOracleBumps, initial_vault_lamports: u64) -> Result<()> {
        // Verify collection authority
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(base_collection.update_authority == self.update_authority.key(), StakingError::InvalidAuthority);

        // Determine current transfer permission based on time
        let current_timestamp = Clock::get()?.unix_timestamp;
        let time_of_day = ((current_timestamp % SECONDS_PER_DAY) + SECONDS_PER_DAY) % SECONDS_PER_DAY;
        let current_hour = time_of_day / 3600;
        let transfer_allowed = current_hour >= OPEN_HOUR && current_hour < CLOSE_HOUR;

        // Initialize oracle account
        self.oracle.set_inner(StakingOracle::init_v1(bumps.oracle, transfer_allowed));

        let collection_key = self.collection.key();
        let signer_seeds = &[
            b"update_authority",
            collection_key.as_ref(),
            &[bumps.update_authority],
        ];

        // Add Oracle external plugin adapter to collection
        // ExternalCheckResult flags: can_reject = bit 2 = 0x4
        AddCollectionExternalPluginAdapterV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .payer(&self.payer.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .init_info(ExternalPluginAdapterInitInfo::Oracle(OracleInitInfo {
                base_address: self.oracle.key(),
                init_plugin_authority: Some(PluginAuthority::UpdateAuthority),
                lifecycle_checks: vec![
                    (HookableLifecycleEvent::Transfer, ExternalCheckResult { flags: 0b100 }), // CAN_REJECT
                ],
                base_address_config: None,
                results_offset: Some(ValidationResultsOffset::Anchor),
            }))
            .invoke_signed(&[signer_seeds])?;

        // Fund vault with initial lamports for crank rewards
        if initial_vault_lamports > 0 {
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    self.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: self.payer.to_account_info(),
                        to: self.vault.to_account_info(),
                    },
                ),
                initial_vault_lamports,
            )?;
        }

        Ok(())
    }
}
