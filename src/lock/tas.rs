use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::guard::TasGuard;
use crate::lock::BorrowError;

use super::{Lock, LockRef, UnboundedLock};

pub struct TasLock { locked: AtomicBool }

impl Lock for TasLock {
    type Ref<'a> = &'a TasLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, BorrowError> {
        Ok(self)
    }
}

impl UnboundedLock for TasLock {
    fn new() -> Self {
        TasLock { locked: AtomicBool::new(false) }
    }
}

impl<'a> LockRef<'a> for &'a TasLock {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let locked = &self.locked;
        while locked.swap(true, Acquire) { };
        TasGuard::new(locked)
    }
}
