use pinocchio::{AccountView, error::ProgramError};

/// Fundraiser state account layout (96 bytes):
/// maker[32] | mint_to_raise[32] | amount_to_raise:u64 | current_amount:u64 |
/// time_started:i64 | duration:u8 | bump:u8 | _padding[6]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fundraiser {
    maker: [u8; 32],
    mint_to_raise: [u8; 32],
    amount_to_raise: [u8; 8],
    current_amount: [u8; 8],
    time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8,
    _padding: [u8; 6],
}

impl Fundraiser {
    pub const LEN: usize = 96;

    pub fn from_account_info(account_info: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = account_info.try_borrow_mut()?;
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn maker(&self) -> pinocchio::Address {
        pinocchio::Address::from(self.maker)
    }

    pub fn set_maker(&mut self, maker: &pinocchio::Address) {
        self.maker.copy_from_slice(maker.as_ref());
    }

    pub fn mint_to_raise(&self) -> pinocchio::Address {
        pinocchio::Address::from(self.mint_to_raise)
    }

    pub fn set_mint_to_raise(&mut self, mint: &pinocchio::Address) {
        self.mint_to_raise.copy_from_slice(mint.as_ref());
    }

    pub fn amount_to_raise(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_raise)
    }

    pub fn set_amount_to_raise(&mut self, amount: u64) {
        self.amount_to_raise = amount.to_le_bytes();
    }

    pub fn current_amount(&self) -> u64 {
        u64::from_le_bytes(self.current_amount)
    }

    pub fn set_current_amount(&mut self, amount: u64) {
        self.current_amount = amount.to_le_bytes();
    }

    pub fn time_started(&self) -> i64 {
        i64::from_le_bytes(self.time_started)
    }

    pub fn set_time_started(&mut self, time: i64) {
        self.time_started = time.to_le_bytes();
    }
}
