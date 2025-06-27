use std::sync::atomic::Ordering::*;
use std::time::{Duration, Instant};

use crate::acqrel::{AcquireBox, RecursiveAcquire};
use crate::atomic::Atomic;
use crate::guard::ReleaseGuard;
use crate::lock::BorrowError;

use super::{Lock, LockRef, UnboundedLock};

pub struct TimeoutLock { tail: Atomic<RecursiveAcquire> }
type TimeoutGuard = ReleaseGuard<Option<RecursiveAcquire>>;

impl TimeoutLock {
    pub fn try_acquire(&self, timeout: Duration) -> Option<TimeoutGuard> {
        let start = Instant::now();
        let (next, mut release) = AcquireBox::default();
        let mut acquire = self.tail.swap(RecursiveAcquire::new(next), Relaxed);
        while let Some(inner_acquire) = acquire.try_recur() {
            if start.elapsed() >= timeout {
                *release = Some(inner_acquire);
                drop(release);
                return None;
            }
            acquire = inner_acquire;
        }
        Some(TimeoutGuard::new(release))
    }
}

impl Lock for TimeoutLock {
    type Ref<'a> = &'a TimeoutLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, BorrowError> {
        Ok(self)
    }
}

impl UnboundedLock for TimeoutLock {
    fn new() -> Self {
        let acquire = RecursiveAcquire::new(AcquireBox::default_acquired());
        TimeoutLock { tail: Atomic::new(acquire) }
    }
}

impl<'a> LockRef<'a> for &'a TimeoutLock {
    type Guard = TimeoutGuard;
    fn acquire(&mut self) -> Self::Guard {
        let (next, release) = AcquireBox::default();
        let acquire = self.tail.swap(RecursiveAcquire::new(next), Relaxed);
        drop(acquire);
        TimeoutGuard::new(release)
    }
}
