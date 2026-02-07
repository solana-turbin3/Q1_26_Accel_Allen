# Transfer Hook Vault

A Solana program that implements a token vault with whitelist-gated access using **Token-2022 Transfer Hooks** and a **Merkle tree** for scalable whitelist management.

Built with Anchor 0.32.1.

## How It Works

The admin creates a Token-2022 mint with a TransferHook extension pointing to this program. A Merkle root stored on-chain represents the set of whitelisted users. Users prove their membership by submitting a Merkle proof, which creates a persistent approval PDA. The transfer hook fires on every `transfer_checked` and verifies the caller has an approval PDA — blocking transfers from non-whitelisted users.

### Deposit / Withdraw Pattern

Because the transfer hook lives in the same program as the vault logic, calling `transfer_checked` via CPI would cause reentrancy. Instead:

- **Deposit**: Client sends two instructions in one transaction — (1) `deposit` (bookkeeping: checks max amount, updates balance) + (2) `transfer_checked` (client-built with hook extra accounts appended).
- **Withdraw**: Client sends two instructions — (1) `withdraw` (bookkeeping + `approve` delegate on vault ATA) + (2) `transfer_checked` with the user as delegate authority.

## State

| Account | Seeds | Description |
|---|---|---|
| `VaultConfig` | `[b"vault_config"]` | Admin, mint, vault ATA address, Merkle root, bumps |
| `WhitelistApproval` | `[b"approval", user_pubkey]` | Per-user approval with max deposit cap and deposited amount |

## Instructions

| Instruction | Who | What |
|---|---|---|
| `initialize` | Admin | Creates VaultConfig, Token-2022 mint (with TransferHook), vault ATA, mints initial supply, creates vault's own approval PDA |
| `initialize_extra_account_meta_list` | Admin | Registers the approval PDA as an ExtraAccountMeta so Token-2022 resolves it during transfers |
| `update_merkle_root` | Admin | Updates the Merkle root (invalidates unclaimed proofs) |
| `claim_whitelist` | User | Submits a Merkle proof → verifies against root → creates WhitelistApproval PDA |
| `revoke_whitelist` | Admin | Closes a user's approval PDA |
| `deposit` | User | Records deposit amount, checks per-user cap. Paired with client-side `transfer_checked` |
| `withdraw` | User | Records withdrawal, approves user as delegate on vault ATA. Paired with client-side `transfer_checked` |
| `transfer_hook` | Token-2022 | Automatically invoked on `transfer_checked`. Verifies the caller's approval PDA exists |

## Merkle Tree

Leaves are `SHA-256(user_pubkey ++ max_deposit_amount_le)`. Each leaf encodes both _who_ is whitelisted and _how much_ they can deposit. The admin builds the tree off-chain and stores only the 32-byte root on-chain.

## Testing

Tests use LiteSVM 0.9.1 (in-process Solana VM, no validator needed).

```sh
anchor build
cargo test-sbf -- --nocapture
```

## Program ID

```
4Uoq2yp6eCji8xx6H7F1SgWWV732TnJhK7rjcyWMp7Fs
```
