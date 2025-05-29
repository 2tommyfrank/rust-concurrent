use crate::atomic::Atomic;
use crate::guard::McsGuard;
use crate::notify::{Notify, Wait};
use crate::Str;

use super::{Lock, LockRef, UnboundedLock};

pub struct McsLock { tail: Atomic<Option<Notify<Option<Notify<()>>>>> }

impl Lock for McsLock {
    type Ref<'a> = &'a McsLock;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
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
        let (wait, notify) = Wait::with(None);
        if let Some(mut notify) = self.tail.swap(Some(notify)) {
            let (inner_wait, inner_notify) = Wait::new();
            *notify = Some(inner_notify);
            drop(notify);
            drop(inner_wait);
        }
        McsGuard::new(&self.tail, wait)
    }
}
