# Whitelist Transfer Hook

A Solana program built with Anchor that enforces whitelist-based transfer restrictions on Token-2022 tokens using the Transfer Hook extension. Only addresses explicitly added to the whitelist by an admin can send tokens.

## Overview

The program registers itself as a transfer hook on Token-2022 mints. Every time a `transfer_checked` is executed on a mint with this hook, Token-2022 automatically invokes the program's `transfer_hook` instruction, which verifies that the sender's address has a corresponding whitelist PDA. If the PDA doesn't exist, the transfer is rejected.

An admin (set during config initialization) manages the whitelist by creating or closing per-address PDA accounts.

### State Accounts

| Account | Seeds | Description |
|---------|-------|-------------|
| `Config` | `["config"]` | Stores the admin pubkey and bump. Single global instance. |
| `WhitelistEntry` | `["whitelist", address]` | One PDA per whitelisted address. Existence = whitelisted. |
| `ExtraAccountMetaList` | `["extra-account-metas", mint]` | TLV account that tells Token-2022 which extra accounts to pass into the hook. |

## Instructions

### `initialize_config`
Creates the global `Config` account with the caller as admin. Can only be called once (PDA is unique).

### `add_to_whitelist(address)`
**Admin only.** Creates a `WhitelistEntry` PDA for the given address, marking it as whitelisted. Fails if the entry already exists.

### `remove_from_whitelist(address)`
**Admin only.** Closes the `WhitelistEntry` PDA for the given address, removing it from the whitelist. Rent is refunded to the admin.

### `create_mint(decimals)`
Creates a new Token-2022 mint with the `TransferHook` extension initialized to point at this program. The caller becomes the mint authority.

### `initialize_extra_account_meta_list`
Registers the extra accounts required by the transfer hook for a given mint. This tells Token-2022 to resolve and pass the sender's `WhitelistEntry` PDA when invoking the hook. Must be called once per mint before transfers can work.

### `transfer_hook(amount)`
Called automatically by Token-2022 during `transfer_checked`. Validates that the source token account is actively transferring and that the sender has a `WhitelistEntry` PDA. If the PDA is missing, the transfer fails with `NotWhitelisted`.
