## Programs

### [Whitelist Transfer Hook](https://github.com/solana-turbin3/Q1_26_Accel_Allen/tree/main/whitelist-transfer-hook)

A Solana program that enforces whitelist-based access control on token transfers using the SPL Token 2022 Transfer Hook interface. Only addresses added to the whitelist by the admin can transfer tokens. Each whitelisted address gets its own PDA account for O(1) lookup during transfer validation.

### [Escrow](https://github.com/solana-turbin3/Q1_26_Accel_Allen/tree/main/escrow-litesvm)

A two-party token escrow program built with Anchor. The maker deposits Token A into a PDA-controlled vault and specifies how much of Token B they want in return. A 5-day time lock is enforced after escrow creation before a taker can accept the offer, atomically swapping Token B for the vault's Token A. The maker can also cancel and reclaim their tokens via a refund instruction. Tests use [LiteSVM](https://github.com/LiteSVM/litesvm) with time travel (warp) for fast, in-process Solana program testing without a local validator.

### [Transfer Hook Vault](https://github.com/solana-turbin3/Q1_26_Accel_Allen/tree/main/transfer-hook-vault)

A Token-2022 vault program with transfer hook enforced whitelist access. The admin initializes a mint with a TransferHook extension and stores a Merkle root representing whitelisted users. Users prove membership via Merkle proof to create a UserState PDA, which the transfer hook checks on every `transfer_checked` — blocking non-whitelisted transfers. Deposits and withdrawals are split into two instructions per transaction (bookkeeping + transfer) to avoid CPI reentrancy. The admin can update the Merkle root or remove users at any time.
