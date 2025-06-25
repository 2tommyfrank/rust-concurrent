use std::sync::atomic::{AtomicBool, Ordering::*};
use std::time::Duration;

use crate::backoff::Backoff;
use crate::guard::TasGuard;
use crate::Str;

use super::{Lock, LockRef, UnboundedLock};

pub struct TtasLock { locked: AtomicBool }

impl TtasLock {
    pub fn try_acquire(&self) -> bool {
        while self.locked.load(Relaxed) { };
        !self.locked.swap(true, Acquire)
    }
}

impl Lock for TtasLock {
    type Ref<'a> = &'a TtasLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for TtasLock {
    fn new() -> Self {
        TtasLock { locked: AtomicBool::new(false) }
    }
}

impl<'a> LockRef<'a> for &'a TtasLock {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        while !self.try_acquire() { };
        TasGuard::new(&self.locked)
    }
}

pub struct BackoffLock {
    ttas: TtasLock,
    min_delay: Duration,
    max_delay: Duration,
}

impl Lock for BackoffLock {
    type Ref<'a> = &'a BackoffLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for BackoffLock {
    fn new() -> Self {
        BackoffLock {
            ttas: TtasLock::new(),
            min_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1000),
        }
    }
}

impl<'a> LockRef<'a> for &'a BackoffLock {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let mut backoff = Backoff::new(self.min_delay, self.max_delay);
        while !self.ttas.try_acquire() { backoff.backoff(); }
        TasGuard::new(&self.ttas.locked)
    }
}
