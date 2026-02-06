use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, sysvar};
use anchor_spl::token_interface::{
    TokenInterface,
    spl_token_2022::{
        extension::{
            ExtensionType,
            transfer_hook::instruction as transfer_hook_ix,
        },
        instruction as token_instruction,
    },
};

use crate::ID;

#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub mint: Signer<'info>,

    #[account(address = sysvar::rent::ID)]
    pub rent: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> CreateMint<'info> {
    pub fn create_mint(&mut self, decimals: u8) -> Result<()> {
        
        let extensions = &[ExtensionType::TransferHook];
        let mint_len = ExtensionType::try_calculate_account_len::<
            anchor_spl::token_interface::spl_token_2022::state::Mint,
        >(extensions)
        .unwrap();

        let lamports = Rent::get()?.minimum_balance(mint_len);

        invoke(
            &anchor_lang::solana_program::system_instruction::create_account(
                &self.payer.key(),
                &self.mint.key(),
                lamports,
                mint_len as u64,
                &self.token_program.key(),
            ),
            &[
                self.payer.to_account_info(),
                self.mint.to_account_info(),
                self.system_program.to_account_info(),
            ],
        )?;

        invoke(
            &transfer_hook_ix::initialize(
                &self.token_program.key(),
                &self.mint.key(),
                Some(self.payer.key()),    
                Some(ID),                  
            )?,
            &[self.mint.to_account_info()],
        )?;

        invoke(
            &token_instruction::initialize_mint(
                &self.token_program.key(),
                &self.mint.key(),
                &self.payer.key(),  
                None,               
                decimals,
            )?,
            &[
                self.mint.to_account_info(),
                self.rent.to_account_info(),
            ],
        )?;

        Ok(())
    }
}