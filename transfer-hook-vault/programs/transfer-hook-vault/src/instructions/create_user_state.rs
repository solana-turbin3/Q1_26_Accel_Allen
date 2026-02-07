use anchor_lang::prelude::*;
use sha2::{Sha256, Digest};

use crate::errors::VaultError;
use crate::state::{VaultConfig, UserState};

#[derive(Accounts)]
pub struct CreateUserState<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"vault_config"],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        init,
        payer = user,
        space = 8 + UserState::INIT_SPACE,
        seeds = [b"user_state", user.key().as_ref()],
        bump,
    )]
    pub user_state: Account<'info, UserState>,

    pub system_program: Program<'info, System>,
}

fn hash_pair(a: &[u8], b: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(a);
    hasher.update(b);
    hasher.finalize().into()
}

fn hash_leaf(pubkey: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(pubkey);
    hasher.finalize().into()
}

impl<'info> CreateUserState<'info> {
    pub fn handler(&mut self, proof: Vec<[u8; 32]>, bumps: &CreateUserStateBumps) -> Result<()> {
        // Compute leaf = sha256(user_pubkey)
        let mut node = hash_leaf(
            self.user.key().as_ref(),
        );

        // Verify Merkle proof
        for proof_element in proof.iter() {
            if node <= *proof_element {
                node = hash_pair(&node, proof_element);
            } else {
                node = hash_pair(proof_element, &node);
            }
        }

        require!(
            node == self.vault_config.merkle_root,
            VaultError::InvalidMerkleProof
        );

        self.user_state.set_inner(UserState {
            user: self.user.key(),
            amount_deposited: 0,
            bump: bumps.user_state,
        });

        msg!("User state created for: {}", self.user.key());
        Ok(())
    }
}
