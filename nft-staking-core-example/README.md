# NFT Staking Core

Non-custodial Metaplex Core NFT staking program built with Anchor. NFTs stay in the user's wallet — staking state is managed through on-chain plugins (FreezeDelegate, Attributes, BurnDelegate, Oracle).

## Features

### Core Staking
- **create_collection** / **mint_nft** — Create a Metaplex Core collection and mint NFTs with a PDA update authority
- **initialize_config** — Set `points_per_stake` (reward rate per day) and `freeze_period` (minimum lock days)
- **stake** — Freeze the NFT via FreezeDelegate, record `staked=true` and `staked_at` timestamp in Attributes plugin, add BurnDelegate for burn-to-earn
- **unstake** — Enforce freeze period, unfreeze NFT, mint reward tokens (days staked × points_per_stake)

### Task 1: Core Plugins
- **claim_rewards** — Collect accumulated rewards without unstaking. Resets `staked_at` to current time to prevent double-claiming. NFT stays staked and frozen.
- **burn_staked_nft** — Permanently burn a staked NFT for a bonus reward. Reward = base (days × rate) + burn bonus (freeze_period × rate × 2). Unfreezes first (FreezeDelegate blocks burns while frozen), then burns via BurnV1 CPI.
- **Collection-level stats** — `total_staked` counter as an Attribute on the Collection account. Incremented on stake, decremented on unstake and burn.

### Task 2: Oracle Plugin
- **create_oracle** — Initializes a StakingOracle PDA whose borsh layout matches `OracleValidation::V1` (read with `ValidationResultsOffset::Anchor`). Adds Oracle external plugin adapter to the collection with `CAN_REJECT` on Transfer lifecycle.
- **crank_oracle** — Permissionless crank that reads the Clock sysvar, determines if current time is within business hours (9AM–5PM UTC), and updates the oracle's transfer field (Pass or Rejected). Requires state to actually change. Pays 0.01 SOL from a vault PDA when cranked within 5 minutes of a boundary.
- **transfer_nft** — Wraps Metaplex `TransferV1` CPI, passing the oracle as a remaining account. Checks `oracle.transfer == PASS` before invoking.
- **fund_vault** — Tops up the vault PDA with lamports for crank rewards.

## Architecture

```
Collection (Metaplex Core)
├── update_authority: PDA [b"update_authority", collection]
├── Attributes plugin: { total_staked: "N" }
└── Oracle external plugin adapter → StakingOracle PDA

Config PDA [b"config", collection]
├── points_per_stake, freeze_period
└── mint authority for rewards_mint PDA [b"rewards", config]

NFT (Metaplex Core Asset)
├── Attributes plugin: { staked: "true/false", staked_at: "<timestamp>" }
├── FreezeDelegate plugin: { frozen: true/false }
└── BurnDelegate plugin (added on first stake)

StakingOracle PDA [b"oracle", collection]
├── OracleValidation V1 borsh layout (variant, create, transfer, burn, update)
└── Transfer: Pass (2) during 9AM-5PM UTC, Rejected (1) outside

Vault PDA [b"vault", collection]
└── Holds lamports for crank rewards
```

### PDA Seeds

| Account | Seeds |
|---|---|
| Update Authority | `["update_authority", collection]` |
| Config | `["config", collection]` |
| Rewards Mint | `["rewards", config]` |
| Oracle | `["oracle", collection]` |
| Vault | `["vault", collection]` |

## Testing

### TypeScript (17 tests) — solana-test-validator
```bash
anchor test
```

### LiteSVM Rust (11 tests) — clock manipulation for time travel
```bash
anchor build
cargo test --package nft-staking-core --test litesvm_test
```

LiteSVM tests verify actual reward calculations by advancing the clock:
- 3 days staked at 10 pts/day → 30 tokens
- Claim 50 after 5 days, claim 30 more after 3 more, unstake for 20 → 100 total
- Burn after 5 days with freeze_period=2: 50 base + 40 burn bonus → 90
- Oracle: transfer allowed at 10AM, blocked at 8PM, re-allowed next day at noon

## Dependencies

```toml
anchor-lang = { version = "0.32.1", features = ["init-if-needed"] }
anchor-spl = "0.32.1"
mpl-core = { version = "0.11.1", features = ["anchor"] }
```

## Program ID

```
BeUozctAJ14QWbNfoFt43VYDFhAaYZFprCHAkAU8y1Wj
```
