use std::sync::atomic::Ordering::*;

use crate::acqrel::{AcquireBox, ReleasePtr};
use crate::atomic::Atomic;
use crate::Str;

use super::{Lock, LockRef, UnboundedLock};

pub struct ClhLock { tail: Atomic<AcquireBox<()>> }

impl Lock for ClhLock {
    type Ref<'a> = &'a ClhLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for ClhLock {
    fn new() -> Self {
        ClhLock { tail: Atomic::new(AcquireBox::default_acquired()) }
    }
}

impl<'a> LockRef<'a> for &'a ClhLock {
    type Guard = ReleasePtr<()>;
    fn acquire(&mut self) -> Self::Guard {
        let (acquire, release) = AcquireBox::default();
        self.tail.swap(acquire, Relaxed);
        release
    }
}
