# Solana GPT Oracle + Tuktuk

Schedule the MagicBlock Solana GPT Oracle via Tuktuk. The program sends a prompt to the GPT oracle on a schedule and receives the LLM response via callback.

## Program ID

`H8Tq9DAw82BcYzeeBpm3BLisK8sQn4Ntyj3AewhNTuvj` (devnet)

## Architecture

### External Programs

- **GPT Oracle** (`LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab`) — MagicBlock's on-chain LLM oracle
- **Tuktuk** (`tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA`) — Helium's task scheduling system

### Instructions

| Instruction | Description |
|---|---|
| `initialize` | Creates `GptConfig` PDA storing admin, oracle context reference, recurring prompt, and latest response |
| `ask_gpt` | CPIs to oracle's `interact_with_llm` with the stored prompt and callback info |
| `receive_response` | Callback invoked by oracle identity PDA — stores LLM response on-chain |
| `schedule_ask_gpt` | Schedules `ask_gpt` via Tuktuk `queue_task_v0` CPI |

### Flow

```
1. Admin calls initialize → creates GptConfig + oracle context
2. Admin calls schedule_ask_gpt (or cron script) → Tuktuk schedules periodic ask_gpt
3. Tuktuk cranker fires → executes ask_gpt → CPI to oracle interact_with_llm
4. Oracle off-chain service processes prompt → calls callback_from_llm
5. Oracle CPIs into receive_response → stores response in GptConfig
```

### Key Design Decisions

- **System-owned payer PDA** (`seeds=[b"payer"]`): Never initialized as an Anchor account so it stays system-owned. This allows it to pay rent for oracle interaction accounts via `invoke_signed` without hitting the "Transfer: from must not carry data" error.
- **Manual oracle CPI**: Instructions are constructed manually (discriminator + borsh) to avoid anchor version conflicts with the oracle crate.
- **Tuktuk integration**: Uses `declare_program!` with the tuktuk IDL (same pattern as transfer-hook-vault).

## Build

```bash
anchor build
```

## Deploy

```bash
solana program deploy target/deploy/solana_gpt_tuktuk.so \
  --program-id target/deploy/solana_gpt_tuktuk-keypair.json \
  --with-compute-unit-price 100000
```

## Cron Setup

```bash
cd cron
npm install
node cron.mjs
```

The cron script creates a Tuktuk task queue and cron job that periodically calls `ask_gpt`. Configurable via env vars:

- `RPC_URL` — defaults to devnet
- `ADMIN_SECRET_KEY` — base58 or JSON array
- `TASK_QUEUE_NAME` — defaults to `solana-gpt-tuktuk`
- `CRON_SCHEDULE` — 7-field cron expression, defaults to `0 */5 * * * * *` (every 5 min)

## Devnet Verification

| Step | Status | Tx |
|---|---|---|
| Program deploy | Confirmed | `3F2VN7W...` |
| Oracle context creation | Confirmed | `3CqrN6e...` |
| `initialize` | Confirmed | `3JjvzDR...` |
| `ask_gpt` CPI to oracle | Confirmed | `41s3zhP...` |
| Tuktuk task queue creation | Confirmed | `2xs1TGF...` |
| Cron job creation | Confirmed | `2pUWb9n...` |
| `receive_response` | Pending — oracle off-chain service inactive |
