# Anchor Escrow

A Solana escrow program built with Anchor that enables trustless token swaps between two parties with a 5-day time lock. Supports both SPL Token and Token-2022 via the Token Interface.

## Overview

A maker creates an escrow by depositing token A into a program-owned vault and specifying how much of token B they want in return. After a 5-day lock period, any taker can fulfill the escrow by sending the requested token B to the maker and receiving the deposited token A from the vault. The maker can cancel and reclaim their tokens at any time via refund.

### State Account

| Account | Seeds | Description |
|---------|-------|-------------|
| `Escrow` | `["escrow", maker, seed]` | Stores maker, both mints, requested receive amount, creation timestamp, and bump. The `seed` (u64) allows a maker to have multiple concurrent escrows. |

The escrow PDA also serves as the authority for the vault token account that holds the deposited tokens.

## Instructions

### `make(seed, deposit, receive)`
Creates an escrow and deposits tokens. Initializes the `Escrow` PDA and a vault token account, then transfers `deposit` amount of mint A from the maker's ATA into the vault. Records `receive` as the amount of mint B expected from the taker.

### `take`
Fulfills the escrow after the 5-day lock period. The taker sends the requested `receive` amount of mint B to the maker, receives all mint A tokens from the vault, and the vault and escrow accounts are closed (rent returned to maker).

### `refund`
**Maker only.** Cancels the escrow at any time. Returns all deposited mint A tokens from the vault back to the maker's ATA, then closes the vault and escrow accounts.

## Testing

Tests use [LiteSVM](https://github.com/LiteSVM/litesvm) — a fast, lightweight Solana VM simulator that runs entirely in-process without needing a local validator. Tests are written in Rust alongside the program code.

**Test cases:**
- **test_make** — verifies escrow creation and token deposit into vault
- **test_take** — full happy path: make, warp clock forward 5 days, take, verify token exchange and account cleanup
- **test_take_before_5_days_fails** — confirms take is rejected before the lock period expires
- **test_refund** — make then refund, verify tokens returned and accounts closed

Run tests with:
```sh
cargo test-sbf
```
