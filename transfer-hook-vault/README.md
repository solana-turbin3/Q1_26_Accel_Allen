# Transfer Hook Vault

A Solana program that implements a token vault with whitelist-gated access using **Token-2022 Transfer Hooks** and a **Merkle tree** for scalable whitelist management. Includes **Tuktuk scheduler** integration for timelocked merkle root updates.

Built with Anchor 0.32.1.

## How It Works

The admin creates a Token-2022 mint with a TransferHook extension pointing to this program. A Merkle root stored on-chain represents the set of whitelisted users. Users prove their membership by submitting a Merkle proof, which creates a persistent approval PDA. The transfer hook fires on every `transfer_checked` and verifies the caller has an approval PDA — blocking transfers from non-whitelisted users.

### Deposit / Withdraw Pattern

Because the transfer hook lives in the same program as the vault logic, calling `transfer_checked` via CPI would cause reentrancy. Instead:

- **Deposit**: Client sends two instructions in one transaction — (1) `deposit` (bookkeeping: updates deposited balance) + (2) `transfer_checked` (client-built with hook extra accounts appended).
- **Withdraw**: Client sends two instructions — (1) `withdraw` (bookkeeping + `approve` delegate on vault ATA) + (2) `transfer_checked` with the user as delegate authority.

### Timelocked Merkle Root Updates (Tuktuk)

Instead of instantly updating the whitelist root, the admin can schedule a delayed update via the [Tuktuk](https://www.tuktuk.fun/) task scheduler:

1. Admin calls `schedule_merkle_root_update` with the new root and a trigger (e.g. `Timestamp(now + 3600)`)
2. The program stores the `pending_merkle_root` on VaultConfig and CPIs into Tuktuk's `queue_task_v0` to schedule execution
3. When the trigger fires, a Tuktuk cranker calls `apply_merkle_root_update`, which moves `pending_merkle_root` → `merkle_root` and clears the pending field

The Tuktuk IDL is integrated via Anchor's `declare_program!` macro, generating type-safe CPI bindings without requiring the `tuktuk-program` crate as a direct dependency.

## State

| Account | Seeds | Description |
|---|---|---|
| `VaultConfig` | `[b"vault_config"]` | Admin, mint, vault ATA address, merkle root, pending merkle root, bump |
| `UserState` | `[b"approval", user_pubkey]` | Per-user whitelist status and deposited amount |

## Instructions

| Instruction | Who | What |
|---|---|---|
| `initialize` | Admin | Creates VaultConfig, Token-2022 mint (with TransferHook), vault ATA, mints initial supply, creates vault's own approval PDA |
| `initialize_extra_account_meta_list` | Admin | Registers the approval PDA as an ExtraAccountMeta so Token-2022 resolves it during transfers |
| `update_merkle_root` | Admin | Instantly updates the Merkle root |
| `schedule_merkle_root_update` | Admin | Stores a pending root and CPIs into Tuktuk to schedule `apply_merkle_root_update` at a future time |
| `apply_merkle_root_update` | Anyone (cranker) | Applies the pending merkle root if one exists. Called by Tuktuk crankers when the trigger fires |
| `create_user_state` | User | Submits a Merkle proof → verifies against root → creates UserState PDA |
| `remove_user` | Admin | Closes a user's UserState PDA, removing whitelist access |
| `deposit` | User | Records deposit amount. Paired with client-side `transfer_checked` |
| `withdraw` | User | Records withdrawal, approves user as delegate on vault ATA. Paired with client-side `transfer_checked` |
| `transfer_hook` | Token-2022 | Automatically invoked on `transfer_checked`. Verifies the caller's UserState PDA exists |

## Merkle Tree

Leaves are `SHA-256(user_pubkey)`. The admin builds the tree off-chain and stores only the 32-byte root on-chain.

## Cron Job

The `cron/` directory contains a TypeScript script that uses `@helium/cron-sdk` to register a recurring Tuktuk cron job calling `apply_merkle_root_update` on a schedule.

```sh
cd cron
npm install
ADMIN_SECRET_KEY=<key> RPC_URL=<url> npm start
```

## Testing

Tests use LiteSVM 0.9.1 (in-process Solana VM, no validator needed).

```sh
anchor build
cargo test
```

## Program ID

```
4Uoq2yp6eCji8xx6H7F1SgWWV732TnJhK7rjcyWMp7Fs
```
