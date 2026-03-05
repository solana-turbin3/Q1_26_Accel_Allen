use anchor_lang::prelude::*;

/// Oracle account whose first bytes (after 8-byte Anchor discriminator) match
/// the borsh layout of `mpl_core::types::OracleValidation::V1`.
/// mpl-core reads this with `ValidationResultsOffset::Anchor`.
#[account]
#[derive(InitSpace)]
pub struct StakingOracle {
    // --- OracleValidation borsh layout (5 bytes) ---
    pub variant: u8,    // 1 = V1
    pub create: u8,     // ExternalValidationResult: 0=Approved, 1=Rejected, 2=Pass
    pub transfer: u8,
    pub burn: u8,
    pub update: u8,
    // --- extra metadata ---
    pub bump: u8,
}

impl StakingOracle {
    pub const PASS: u8 = 2;
    pub const REJECTED: u8 = 1;

    pub fn init_v1(bump: u8, transfer_allowed: bool) -> Self {
        Self {
            variant: 1, // V1
            create: Self::PASS,
            transfer: if transfer_allowed { Self::PASS } else { Self::REJECTED },
            burn: Self::PASS,
            update: Self::PASS,
            bump,
        }
    }
}
