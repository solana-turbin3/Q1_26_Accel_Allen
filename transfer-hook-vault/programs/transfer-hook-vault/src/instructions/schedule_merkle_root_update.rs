use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};

use crate::errors::VaultError;
use crate::state::VaultConfig;
use crate::tuktuk_types::{tuktuk, compile_transaction, TUKTUK_PROGRAM_ID};

#[derive(Accounts)]
#[instruction(new_root: [u8; 32], task_id: u16)]
pub struct ScheduleMerkleRootUpdate<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault_config"],
        bump = vault_config.bump,
        constraint = vault_config.admin == admin.key() @ VaultError::Unauthorized,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    /// CHECK: Queue authority PDA owned by this program — signs the CPI
    #[account(
        seeds = [b"queue_authority"],
        bump,
    )]
    pub queue_authority: UncheckedAccount<'info>,

    /// CHECK: Tuktuk task_queue_authority PDA — validated by tuktuk program
    pub task_queue_authority: UncheckedAccount<'info>,

    /// CHECK: Tuktuk task queue account
    #[account(mut)]
    pub task_queue: UncheckedAccount<'info>,

    /// CHECK: Tuktuk task PDA — derived from task_queue + task_id
    #[account(mut)]
    pub task: UncheckedAccount<'info>,

    /// CHECK: Tuktuk program
    #[account(address = TUKTUK_PROGRAM_ID)]
    pub tuktuk_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ScheduleMerkleRootUpdate<'info> {
    pub fn handler(
        &mut self,
        new_root: [u8; 32],
        task_id: u16,
        trigger: tuktuk::types::TriggerV0,
        bumps: &ScheduleMerkleRootUpdateBumps,
    ) -> Result<()> {
        // Store the pending root
        self.vault_config.pending_merkle_root = new_root;

        // Build the apply_merkle_root_update instruction that tuktuk will execute
        let apply_ix = Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta {
                    pubkey: self.vault_config.key(),
                    is_signer: false,
                    is_writable: true,
                },
            ],
            data: crate::instruction::ApplyMerkleRootUpdate {}.data(),
        };

        let (compiled_tx, remaining_accounts) = compile_transaction(
            vec![apply_ix],
            vec![],
        )?;

        let args = tuktuk::types::QueueTaskArgsV0 {
            id: task_id,
            trigger,
            transaction: tuktuk::types::TransactionSourceV0::CompiledV0(compiled_tx),
            crank_reward: None,
            free_tasks: 0,
            description: "apply_merkle_root".to_string(),
        };

        // Build the queue_task_v0 CPI instruction using the IDL discriminator
        let disc = tuktuk::client::args::QueueTaskV0::DISCRIMINATOR;
        let mut ix_data = disc.to_vec();
        AnchorSerialize::serialize(&args, &mut ix_data)?;

        let mut cpi_accounts = vec![
            AccountMeta { pubkey: self.admin.key(), is_signer: true, is_writable: true },
            AccountMeta { pubkey: self.queue_authority.key(), is_signer: true, is_writable: false },
            AccountMeta { pubkey: self.task_queue_authority.key(), is_signer: false, is_writable: false },
            AccountMeta { pubkey: self.task_queue.key(), is_signer: false, is_writable: true },
            AccountMeta { pubkey: self.task.key(), is_signer: false, is_writable: true },
            AccountMeta { pubkey: self.system_program.key(), is_signer: false, is_writable: false },
        ];
        cpi_accounts.extend(remaining_accounts);

        let cpi_ix = Instruction {
            program_id: TUKTUK_PROGRAM_ID,
            accounts: cpi_accounts,
            data: ix_data,
        };

        let signer_seeds: &[&[u8]] = &[b"queue_authority", &[bumps.queue_authority]];

        invoke_signed(
            &cpi_ix,
            &[
                self.admin.to_account_info(),
                self.queue_authority.to_account_info(),
                self.task_queue_authority.to_account_info(),
                self.task_queue.to_account_info(),
                self.task.to_account_info(),
                self.system_program.to_account_info(),
                self.tuktuk_program.to_account_info(),
                // Pass vault_config as remaining account (referenced in compiled tx)
                self.vault_config.to_account_info(),
            ],
            &[signer_seeds],
        )?;

        msg!("Merkle root update scheduled (task_id={})", task_id);
        Ok(())
    }
}
