use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct FundVault<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    /// CHECK: Collection (for PDA derivation)
    pub collection: UncheckedAccount<'info>,
    /// CHECK: Vault PDA holding lamports for crank rewards
    #[account(
        mut,
        seeds = [b"vault", collection.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> FundVault<'info> {
    pub fn fund_vault(&self, amount: u64) -> Result<()> {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: self.funder.to_account_info(),
                    to: self.vault.to_account_info(),
                },
            ),
            amount,
        )?;
        Ok(())
    }
}
