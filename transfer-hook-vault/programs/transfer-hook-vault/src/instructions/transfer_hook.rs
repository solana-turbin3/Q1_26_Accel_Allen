use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount,
            BaseStateWithExtensions,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{Mint, TokenAccount},
};

use crate::errors::VaultError;

#[derive(Accounts)]
pub struct TransferHookCtx<'info> {
    #[account(
        token::mint = mint,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: source token account owner
    pub owner: UncheckedAccount<'info>,

    /// CHECK: ExtraAccountMetaList PDA
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    /// CHECK: UserState PDA for the owner — checked manually
    #[account(
        seeds = [b"user_state", owner.key().as_ref()],
        bump,
    )]
    pub user_state: UncheckedAccount<'info>,
}

impl<'info> TransferHookCtx<'info> {
    pub fn handler(&self, _amount: u64) -> Result<()> {
        self.check_is_transferring()?;

        // Check that user_state PDA exists (has data and is owned by our program)
        let user_state_info = &self.user_state;
        require!(
            user_state_info.owner == &crate::ID && user_state_info.data_len() > 0,
            VaultError::NotWhitelisted
        );

        msg!("Transfer hook: {} is approved", self.owner.key());
        Ok(())
    }

    fn check_is_transferring(&self) -> Result<()> {
        let source_token_info = self.source_token.to_account_info();
        let mut account_data_ref: RefMut<&mut [u8]> =
            source_token_info.try_borrow_mut_data()?;

        let account =
            PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let extension = account.get_extension::<TransferHookAccount>()?;

        require!(
            bool::from(extension.transferring),
            VaultError::NotTransferring
        );

        Ok(())
    }
}
