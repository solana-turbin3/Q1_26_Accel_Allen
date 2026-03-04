# Pinocchio Fundraiser

A Solana fundraiser program built with [Pinocchio](https://github.com/anza-xyz/pinocchio) — a zero-dependency, zero-allocation native Solana framework. This is a port of the Anchor-based fundraiser, optimized for minimal compute unit usage.

## Overview

The program allows a **maker** to create a fundraiser targeting a specific SPL token amount. **Contributors** can donate tokens (capped at 10% of the target per contributor). If the target is met, the maker can claim the funds. If the fundraiser expires without meeting its target, contributors can reclaim their tokens.

## Program Architecture

- **Custom BPF entrypoint** with single-byte discriminator dispatch
- **`#[repr(C)]` state** with raw pointer cast load/store
- **Zero heap allocation** — no `Vec`, `String`, or `Box`
- **Raw CPI** via Pinocchio's typed instruction builders

## State Accounts

### Fundraiser (96 bytes)

| Field | Type | Offset | Description |
|-------|------|--------|-------------|
| `maker` | Pubkey | 0 | Creator of the fundraiser |
| `mint_to_raise` | Pubkey | 32 | SPL token mint to raise |
| `amount_to_raise` | u64 | 64 | Target amount in token base units |
| `current_amount` | u64 | 72 | Running total of contributions |
| `time_started` | i64 | 80 | Unix timestamp of creation |
| `duration` | u8 | 88 | Fundraiser duration in days |
| `bump` | u8 | 89 | PDA bump seed |
| `_padding` | [u8; 6] | 90 | Alignment padding |

**PDA seeds:** `["fundraiser", maker_pubkey]`

### Contributor (48 bytes)

| Field | Type | Offset | Description |
|-------|------|--------|-------------|
| `contributor` | Pubkey | 0 | Contributor's public key |
| `amount` | u64 | 32 | Total amount contributed |
| `bump` | u8 | 40 | PDA bump seed |
| `_padding` | [u8; 7] | 41 | Alignment padding |

**PDA seeds:** `["contributor", fundraiser_pubkey, contributor_pubkey]`

## Instructions

### 0 — Initialize

Creates a new fundraiser. The maker specifies the target amount and duration.

| Account | Signer | Mutable | Description |
|---------|--------|---------|-------------|
| `maker` | Yes | Yes | Fundraiser creator, pays for account |
| `fundraiser` | No | Yes | Fundraiser PDA (created) |
| `mint_to_raise` | No | No | SPL token mint |
| `system_program` | No | No | System program |

**Data:** `amount_to_raise: u64, duration: u8, bump: u8`

**Validation:** `amount_to_raise > 3^mint_decimals`

### 1 — Create Contributor

Creates a contributor state PDA. Separated from the contribute instruction to avoid `init_if_needed` patterns.

| Account | Signer | Mutable | Description |
|---------|--------|---------|-------------|
| `contributor` | Yes | Yes | Contributor, pays for account |
| `fundraiser` | No | No | Fundraiser PDA |
| `contributor_state` | No | Yes | Contributor PDA (created) |
| `system_program` | No | No | System program |

**Data:** `bump: u8`

### 2 — Contribute

Transfers tokens from the contributor to the fundraiser vault.

| Account | Signer | Mutable | Description |
|---------|--------|---------|-------------|
| `contributor` | Yes | No | Token contributor |
| `fundraiser` | No | Yes | Fundraiser PDA (state updated) |
| `vault` | No | Yes | Fundraiser's token vault ATA |
| `contributor_ata` | No | Yes | Contributor's token ATA |
| `contributor_state` | No | Yes | Contributor PDA (amount updated) |
| `token_program` | No | No | SPL Token program |

**Data:** `amount: u64`

**Validation:**
- `amount > 0`
- `amount <= 10%` of target (per-transaction cap)
- Fundraiser duration not exceeded
- Contributor's cumulative total stays within 10% cap

### 3 — Checker

Verifies the fundraiser target is met, transfers all vault tokens to the maker, and closes the fundraiser account.

| Account | Signer | Mutable | Description |
|---------|--------|---------|-------------|
| `maker` | Yes | Yes | Fundraiser creator (receives lamports) |
| `fundraiser` | No | Yes | Fundraiser PDA (closed) |
| `vault` | No | Yes | Token vault ATA |
| `maker_ata` | No | Yes | Maker's token ATA (receives tokens) |
| `token_program` | No | No | SPL Token program |

**Data:** none

**Validation:** `vault_balance >= amount_to_raise`

### 4 — Refund

Refunds a contributor if the fundraiser has expired without meeting its target. Closes the contributor state account.

| Account | Signer | Mutable | Description |
|---------|--------|---------|-------------|
| `contributor` | Yes | Yes | Contributor (receives lamports + tokens) |
| `maker` | No | No | Fundraiser creator (for PDA derivation) |
| `fundraiser` | No | Yes | Fundraiser PDA (current_amount updated) |
| `vault` | No | Yes | Token vault ATA |
| `contributor_ata` | No | Yes | Contributor's token ATA (receives refund) |
| `contributor_state` | No | Yes | Contributor PDA (closed) |
| `token_program` | No | No | SPL Token program |

**Data:** none

**Validation:**
- Days elapsed > duration (fundraiser expired)
- Vault balance < target (target not met)

## Testing

Tests use [LiteSVM](https://github.com/LiteSVM/litesvm) for fast, local Solana program testing.

```bash
# Build the SBF binary first
cargo build-sbf

# Run all tests
cargo test
```

### Test Coverage

| Test | Description |
|------|-------------|
| `test_initialize` | Creates fundraiser PDA, verifies 96-byte account |
| `test_create_contributor` | Creates contributor PDA, verifies 48-byte account |
| `test_contribute` | Contributes tokens, verifies vault balance |
| `test_checker` | Full flow: 10 contributors meet target, maker claims funds, fundraiser closed |
| `test_refund` | Contribute, warp clock past duration, refund, verify tokens returned |

## Dependencies

```toml
pinocchio = "0.10.2"
pinocchio-pubkey = "0.3.0"
pinocchio-system = "0.5.0"
pinocchio-token = "0.5.0"
```
