use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, sysvar};
use anchor_spl::token_interface::{
    TokenInterface,
    spl_token_2022::{
        extension::ExtensionType,
        instruction as token_instruction,
        extension::transfer_hook::instruction as transfer_hook_ix,
    },
};

use crate::state::{VaultConfig, UserState};
use crate::ID;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [b"vault_config"],
        bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    /// UserState PDA for vault_config so the transfer hook passes on withdrawals
    #[account(
        init,
        payer = admin,
        space = 8 + UserState::INIT_SPACE,
        seeds = [b"user_state", vault_config.key().as_ref()],
        bump,
    )]
    pub vault_user_state: Account<'info, UserState>,

    /// CHECK: Mint account — initialized via CPI in handler
    #[account(mut)]
    pub mint: Signer<'info>,

    /// CHECK: Vault token account — created via CPI as ATA
    #[account(mut)]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: Associated token program
    #[account(address = anchor_spl::associated_token::ID)]
    pub associated_token_program: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: Rent sysvar — validated by address constraint
    #[account(address = sysvar::rent::ID)]
    pub rent: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handler(&mut self, merkle_root: [u8; 32], initial_supply: u64, bumps: &InitializeBumps) -> Result<()> {
        let decimals: u8 = 6;

        // Calculate mint account size with TransferHook extension
        let extensions = &[ExtensionType::TransferHook];
        let mint_len = ExtensionType::try_calculate_account_len::<
            anchor_spl::token_interface::spl_token_2022::state::Mint,
        >(extensions)
        .unwrap();

        let lamports = Rent::get()?.minimum_balance(mint_len);

        // Create mint account
        invoke(
            &anchor_lang::solana_program::system_instruction::create_account(
                &self.admin.key(),
                &self.mint.key(),
                lamports,
                mint_len as u64,
                &self.token_program.key(),
            ),
            &[
                self.admin.to_account_info(),
                self.mint.to_account_info(),
                self.system_program.to_account_info(),
            ],
        )?;

        // Initialize TransferHook extension
        invoke(
            &transfer_hook_ix::initialize(
                &self.token_program.key(),
                &self.mint.key(),
                Some(self.admin.key()),
                Some(ID),
            )?,
            &[self.mint.to_account_info()],
        )?;

        // Initialize the mint
        invoke(
            &token_instruction::initialize_mint(
                &self.token_program.key(),
                &self.mint.key(),
                &self.admin.key(),
                None,
                decimals,
            )?,
            &[
                self.mint.to_account_info(),
                self.rent.to_account_info(),
            ],
        )?;

        // Create vault ATA via CPI to associated token program
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                &self.admin.key(),
                &self.vault_config.key(),
                &self.mint.key(),
                &self.token_program.key(),
            ),
            &[
                self.admin.to_account_info(),
                self.vault.to_account_info(),
                self.vault_config.to_account_info(),
                self.mint.to_account_info(),
                self.system_program.to_account_info(),
                self.token_program.to_account_info(),
                self.associated_token_program.to_account_info(),
            ],
        )?;

        // Mint initial supply to vault
        if initial_supply > 0 {
            invoke(
                &token_instruction::mint_to(
                    &self.token_program.key(),
                    &self.mint.key(),
                    &self.vault.key(),
                    &self.admin.key(),
                    &[],
                    initial_supply,
                )?,
                &[
                    self.mint.to_account_info(),
                    self.vault.to_account_info(),
                    self.admin.to_account_info(),
                ],
            )?;
        }

        // Derive vault ATA address
        let (vault_ata, _) = Pubkey::find_program_address(
            &[
                self.vault_config.key().as_ref(),
                self.token_program.key().as_ref(),
                self.mint.key().as_ref(),
            ],
            &anchor_spl::associated_token::ID,
        );

        self.vault_config.set_inner(VaultConfig {
            admin: self.admin.key(),
            mint: self.mint.key(),
            vault: vault_ata,
            merkle_root,
            pending_merkle_root: [0u8; 32],
            bump: bumps.vault_config,
        });

        // Create UserState PDA for vault_config so hook passes on withdrawals
        self.vault_user_state.set_inner(UserState {
            user: self.vault_config.key(),
            amount_deposited: 0,
            bump: bumps.vault_user_state,
        });

        msg!("Vault initialized. Admin: {}, Mint: {}", self.admin.key(), self.mint.key());
        Ok(())
    }
}
