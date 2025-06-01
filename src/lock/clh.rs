use crate::atomic::Atomic;
use crate::notify::{Notify, Wait};
use crate::Str;

use super::{Lock, LockRef, UnboundedLock};

pub struct ClhLock { tail: Atomic<Box<Wait<()>>> }

impl Lock for ClhLock {
    type Ref<'a> = &'a ClhLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for ClhLock {
    fn new() -> Self {
        ClhLock { tail: Atomic::new(Wait::already_notified()) }
    }
}

impl<'a> LockRef<'a> for &'a ClhLock {
    type Guard = Notify<()>;
    fn acquire(&mut self) -> Self::Guard {
        let (wait, notify) = Wait::new();
        self.tail.swap(wait);
        notify
    }
}
