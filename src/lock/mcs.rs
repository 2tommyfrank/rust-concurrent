use std::sync::atomic::Ordering::*;

use crate::acqrel::{AcquireBox, ReleasePtr};
use crate::atomic::Atomic;
use crate::guard::McsGuard;
use crate::Str;

use super::{Lock, LockRef, UnboundedLock};

pub struct McsLock { tail: Atomic<Option<ReleasePtr<Option<ReleasePtr<()>>>>> }

impl Lock for McsLock {
    type Ref<'a> = &'a McsLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for McsLock {
    fn new() -> Self {
        McsLock { tail: Atomic::new(None) }
    }
}

impl<'a> LockRef<'a> for &'a McsLock {
    type Guard = McsGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let (acquire, release) = AcquireBox::new(None);
        if let Some(mut release) = self.tail.swap(Some(release), Relaxed) {
            let (inner_acquire, inner_release) = AcquireBox::default();
            *release = Some(inner_release);
            drop(release);
            drop(inner_acquire);
        }
        McsGuard::new(&self.tail, acquire)
    }
}
