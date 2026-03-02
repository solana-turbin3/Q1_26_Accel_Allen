# Pinocchio Escrow

A trustless SPL token escrow program built with [Pinocchio](https://github.com/anza-xyz/pinocchio), a zero-dependency Solana program framework focused on minimal compute unit usage.

Program ID: `4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT`

## Overview

A maker deposits token A into a PDA-owned vault and specifies how much of token B they want in return. Any taker can fulfill the escrow by sending token B to the maker and receiving token A from the vault. The maker can cancel at any time to reclaim their tokens.

## Instructions

### Make (discriminator: 0)

Creates an escrow and deposits tokens into a PDA-owned vault.

**Instruction data:** `[bump: u8, amount_to_receive: u64, amount_to_give: u64]` (raw pointer cast)

**Accounts:**
| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | maker | yes | yes | Escrow creator, pays for account creation |
| 1 | mint_a | no | yes | Mint of the token being deposited |
| 2 | mint_b | no | yes | Mint of the token the maker wants to receive |
| 3 | escrow | no | yes | PDA: `["escrow", maker]` |
| 4 | maker_ata | no | yes | Maker's ATA for mint_a (source) |
| 5 | vault | no | yes | Escrow PDA's ATA for mint_a (created here) |
| 6 | system_program | no | no | System Program |
| 7 | token_program | no | no | SPL Token Program |
| 8 | associated_token_program | no | no | Associated Token Program |

**Flow:**
1. Validates maker_ata ownership and mint
2. Derives and verifies escrow PDA
3. Creates escrow account via `CreateAccount` CPI (PDA-signed)
4. Writes escrow state (maker, mint_a, mint_b, amounts, bump)
5. Creates vault ATA owned by escrow PDA
6. Transfers `amount_to_give` of mint_a from maker_ata to vault

### MakeV2 (discriminator: 3)

Same logic as Make but uses [Wincode](https://github.com/anza-xyz/wincode) for safe deserialization instead of raw `unsafe` pointer casts.

**Instruction data:** same layout, deserialized via `wincode::deserialize`

### Take (discriminator: 1)

Fulfills an existing escrow. The taker sends token B to the maker and receives token A from the vault.

**Accounts:**
| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | taker | yes | yes | Fulfills the escrow |
| 1 | maker | no | yes | Original escrow creator (receives lamports back) |
| 2 | mint_a | no | no | Mint of the deposited token |
| 3 | mint_b | no | no | Mint of the requested token |
| 4 | escrow | no | yes | Escrow PDA |
| 5 | taker_ata_a | no | yes | Taker's ATA for mint_a (receives tokens) |
| 6 | taker_ata_b | no | yes | Taker's ATA for mint_b (sends tokens) |
| 7 | maker_ata_b | no | yes | Maker's ATA for mint_b (receives tokens) |
| 8 | vault | no | yes | Escrow PDA's ATA for mint_a |
| 9 | system_program | no | no | System Program |
| 10 | token_program | no | no | SPL Token Program |
| 11 | associated_token_program | no | no | Associated Token Program |

**Flow:**
1. Loads escrow state, verifies maker address and both mints
2. Transfers `amount_to_receive` of mint_b: taker_ata_b to maker_ata_b (taker signs)
3. Transfers `amount_to_give` of mint_a: vault to taker_ata_a (escrow PDA signs)
4. Closes vault ATA, sends rent to maker (escrow PDA signs)
5. Closes escrow account, returns lamports to maker

### Cancel (discriminator: 2)

Allows the maker to cancel and reclaim deposited tokens.

**Accounts:**
| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | maker | yes | yes | Must match the escrow's stored maker |
| 1 | mint_a | no | no | Mint of the deposited token |
| 2 | escrow | no | yes | Escrow PDA |
| 3 | maker_ata | no | yes | Maker's ATA for mint_a (receives tokens back) |
| 4 | vault | no | yes | Escrow PDA's ATA for mint_a |
| 5 | token_program | no | no | SPL Token Program |
| 6 | system_program | no | no | System Program |

**Flow:**
1. Verifies maker is signer and matches escrow state
2. Transfers all mint_a from vault back to maker_ata (escrow PDA signs)
3. Closes vault ATA, sends rent to maker (escrow PDA signs)
4. Closes escrow account, returns lamports to maker

## Escrow State Layout

```
Offset  Size  Field
0       32    maker (pubkey)
32      32    mint_a (pubkey)
64      32    mint_b (pubkey)
96      8     amount_to_receive (u64 LE)
104     8     amount_to_give (u64 LE)
112     1     bump (u8)
Total: 113 bytes
```

PDA seeds: `["escrow", maker_pubkey]`

## Building

```bash
cargo build-sbf
```

## Testing

Tests use LiteSVM to run the program in a simulated Solana runtime.

```bash
cargo build-sbf && cargo test
```

## Key Pinocchio Patterns

- **AccountView borrow management**: `TokenAccount::from_account_view` returns a `Ref<T>` that holds the account borrow. Must be dropped (via scoping) before any CPI that touches the same account, or the runtime returns `AccountBorrowFailed`.
- **PDA signing**: Build `Seed` array, wrap in `Signer`, pass to `invoke_signed`.
- **Manual account closing**: Zero account data, transfer all lamports to destination, set lamports to 0. No framework helper needed.
- **Wincode deserialization** (MakeV2): Drop-in replacement for unsafe pointer casts. Same wire format, safer code.

## Dependencies

| Crate | Purpose |
|-------|---------|
| pinocchio | Core program framework (entrypoint, AccountView, CPI) |
| pinocchio-token | SPL Token instruction builders and state deserialization |
| pinocchio-system | System Program CPI (CreateAccount) |
| pinocchio-associated-token-account | ATA creation CPI |
| pinocchio-pubkey | PDA derivation (`derive_address`) |
| wincode | Safe binary (de)serialization for instruction data |
