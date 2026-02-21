# Q1 2026 Accelerator

Solana programs built with Anchor during the Turbin3 Q1 2026 accelerator.

## Programs

| Program | Description |
|---------|-------------|
| [escrow-litesvm](./escrow-litesvm) | Trustless token escrow with 5-day time lock. Supports SPL Token and Token-2022 via Token Interface. Tested with LiteSVM. |
| [whitelist-transfer-hook](./whitelist-transfer-hook) | Token-2022 Transfer Hook that enforces whitelist-based transfer restrictions. Admin manages per-address whitelist PDAs. |
| [transfer-hook-vault](./transfer-hook-vault) | Token vault with whitelist-gated access using Token-2022 Transfer Hooks and Merkle tree. Includes Tuktuk scheduler for timelocked merkle root updates. |
| [solana-gpt-tuktuk](./solana-gpt-tuktuk) | Schedules MagicBlock's Solana GPT Oracle via Tuktuk. Sends prompts on a cron schedule and stores LLM responses on-chain via callback. |
