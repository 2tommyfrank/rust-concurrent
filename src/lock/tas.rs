use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::guard::TasGuard;
use crate::Str;

use super::{Lock, LockRef, UnboundedLock};

pub struct TasLock { locked: AtomicBool }

impl Lock for TasLock {
    type Ref<'a> = &'a TasLock;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
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
        while locked.swap(true, Acquire) {};
        TasGuard::new(locked)
    }
}
