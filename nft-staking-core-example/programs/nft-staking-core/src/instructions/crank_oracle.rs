use anchor_lang::prelude::*;
use crate::state::StakingOracle;
use crate::errors::StakingError;

const SECONDS_PER_DAY: i64 = 86400;
const OPEN_HOUR: i64 = 9;   // 9 AM UTC
const CLOSE_HOUR: i64 = 17; // 5 PM UTC
const BOUNDARY_WINDOW_SECONDS: i64 = 300; // 5 minute window around boundaries
const CRANK_REWARD_LAMPORTS: u64 = 10_000_000; // 0.01 SOL

#[derive(Accounts)]
pub struct CrankOracle<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    /// CHECK: Collection (for PDA derivation)
    pub collection: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"oracle", collection.key().as_ref()],
        bump = oracle.bump,
    )]
    pub oracle: Account<'info, StakingOracle>,
    /// CHECK: Vault PDA holding lamports for crank rewards
    #[account(
        mut,
        seeds = [b"vault", collection.key().as_ref()],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CrankOracle<'info> {
    pub fn crank_oracle(&mut self, bumps: &CrankOracleBumps) -> Result<()> {
        let current_timestamp = Clock::get()?.unix_timestamp;
        let time_of_day = ((current_timestamp % SECONDS_PER_DAY) + SECONDS_PER_DAY) % SECONDS_PER_DAY;
        let current_hour = time_of_day / 3600;

        let transfer_allowed = current_hour >= OPEN_HOUR && current_hour < CLOSE_HOUR;
        let new_transfer = if transfer_allowed { StakingOracle::PASS } else { StakingOracle::REJECTED };

        // Verify state actually changes
        let old_transfer = self.oracle.transfer;
        require!(old_transfer != new_transfer, StakingError::OracleStateUnchanged);

        // Update oracle validation
        self.oracle.transfer = new_transfer;

        // Check if near a boundary (within BOUNDARY_WINDOW_SECONDS of 9AM or 5PM)
        let open_seconds = OPEN_HOUR * 3600;
        let close_seconds = CLOSE_HOUR * 3600;

        let near_open = (time_of_day - open_seconds).abs() <= BOUNDARY_WINDOW_SECONDS;
        let near_close = (time_of_day - close_seconds).abs() <= BOUNDARY_WINDOW_SECONDS;

        if (near_open || near_close) && self.vault.lamports() >= CRANK_REWARD_LAMPORTS {
            let collection_key = self.collection.key();
            let vault_seeds: &[&[u8]] = &[
                b"vault",
                collection_key.as_ref(),
                &[bumps.vault],
            ];

            anchor_lang::system_program::transfer(
                CpiContext::new_with_signer(
                    self.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: self.vault.to_account_info(),
                        to: self.cranker.to_account_info(),
                    },
                    &[vault_seeds],
                ),
                CRANK_REWARD_LAMPORTS,
            )?;
        }

        Ok(())
    }
}
