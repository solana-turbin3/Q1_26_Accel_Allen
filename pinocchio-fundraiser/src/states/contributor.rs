use pinocchio::{AccountView, error::ProgramError};

/// Contributor state account layout (48 bytes):
/// contributor[32] | amount:u64 | bump:u8 | _padding[7]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contributor {
    contributor: [u8; 32],
    amount: [u8; 8],
    pub bump: u8,
    _padding: [u8; 7],
}

impl Contributor {
    pub const LEN: usize = 48;

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

    pub fn contributor(&self) -> pinocchio::Address {
        pinocchio::Address::from(self.contributor)
    }

    pub fn set_contributor(&mut self, contributor: &pinocchio::Address) {
        self.contributor.copy_from_slice(contributor.as_ref());
    }

    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.amount = amount.to_le_bytes();
    }
}
