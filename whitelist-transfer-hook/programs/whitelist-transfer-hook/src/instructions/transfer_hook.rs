use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount,
            BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{Mint, TokenAccount},
};

use crate::errors::WhitelistError;
use crate::states::WhitelistEntry;

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint,
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,

    /// CHECK: ExtraAccountMetaList PDA — validated by seeds
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    #[account(
        seeds = [b"whitelist", owner.key().as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
}

impl<'info> TransferHook<'info> {
    pub fn transfer_hook(&self, _amount: u64) -> Result<()> {
        self.check_is_transferring()?;
        msg!(
            "Transfer allowed: {} is whitelisted",
            self.owner.key()
        );
        Ok(())
    }

    fn check_is_transferring(&self) -> Result<()> {
        let source_token_info = self.source_token.to_account_info();
        let mut account_data_ref: RefMut<&mut [u8]> =
            source_token_info.try_borrow_mut_data()?;

        let mut account =
            PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let extension = account.get_extension_mut::<TransferHookAccount>()?;

        require!(
            bool::from(extension.transferring),
            WhitelistError::NotTransferring
        );

        Ok(())
    }
}