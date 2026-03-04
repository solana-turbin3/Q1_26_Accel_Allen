# Q1 2026 Accelerator

Solana programs built with Anchor during the Turbin3 Q1 2026 accelerator.

## Programs

| Program | Description |
|---------|-------------|
| [escrow-litesvm](./escrow-litesvm) | Trustless token escrow with 5-day time lock. Supports SPL Token and Token-2022 via Token Interface. Tested with LiteSVM. |
| [whitelist-transfer-hook](./whitelist-transfer-hook) | Token-2022 Transfer Hook that enforces whitelist-based transfer restrictions. Admin manages per-address whitelist PDAs. |
| [transfer-hook-vault](./transfer-hook-vault) | Token vault with whitelist-gated access using Token-2022 Transfer Hooks and Merkle tree. Includes Tuktuk scheduler for timelocked merkle root updates. |
| [solana-gpt-tuktuk](./solana-gpt-tuktuk) | Schedules MagicBlock's Solana GPT Oracle via Tuktuk. Sends prompts on a cron schedule and stores LLM responses on-chain via callback. |
| [generic-storage](./generic_storage) | Rust learning project: format-agnostic storage system using traits, generics, and PhantomData with Borsh, Wincode, and JSON serializers. |
| [todo-queue](./todo_queue) | Rust learning project: CLI todo app with a generic FIFO queue, Borsh-only persistence, and Clap subcommands. |
| [pinocchio-escrow](./accel-pinocchio-escrow) | Trustless SPL token escrow built with Pinocchio (zero-dependency framework). Supports Make, Take, Cancel, and a MakeV2 using Wincode deserialization. Tested with LiteSVM. |
| [pinocchio-fundraiser](./pinocchio-fundraiser) | SPL token fundraiser built with Pinocchio. Supports Initialize, CreateContributor, Contribute, Checker (claim), and Refund. 10% per-contributor cap with time-based expiry. Tested with LiteSVM. |
