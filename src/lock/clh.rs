use std::sync::atomic::Ordering::*;

use crate::acqrel::AcquireBox;
use crate::atomic::Atomic;
use crate::guard::ReleaseGuard;
use crate::lock::BorrowError;

use super::{Lock, LockRef, UnboundedLock};

pub struct ClhLock { tail: Atomic<AcquireBox<()>> }

impl Lock for ClhLock {
    type Ref<'a> = &'a ClhLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, BorrowError> {
        Ok(self)
    }
}

impl UnboundedLock for ClhLock {
    fn new() -> Self {
        ClhLock { tail: Atomic::new(AcquireBox::default_acquired()) }
    }
}

impl<'a> LockRef<'a> for &'a ClhLock {
    type Guard = ReleaseGuard<()>;
    fn acquire(&mut self) -> Self::Guard {
        let (next, release) = AcquireBox::default();
        let acquire = self.tail.swap(next, Relaxed);
        drop(acquire);
        ReleaseGuard::new(release)
    }
}
