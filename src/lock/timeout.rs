use std::ptr::NonNull;
use std::sync::atomic::Ordering::*;
use std::time::{Duration, Instant};

use crate::acqrel::{Transferable, AcquireBox, ReleasePtr};
use crate::atomic::Atomic;
use crate::raw::Raw;
use crate::Str;

use super::{Lock, LockRef, UnboundedLock};

pub struct RecursiveAcquire(AcquireBox<Option<RecursiveAcquire>>);

impl RecursiveAcquire {
    pub fn try_recur(mut self) -> Option<Self> {
        match self.0.try_as_mut() {
            Ok(next) => next.take().and_then(Self::try_recur),
            Err(()) => Some(self),
        }
    }
}

impl Raw for RecursiveAcquire {
    type Target = NonNull<Transferable<Option<RecursiveAcquire>>>;
    fn as_raw(&self) -> Self::Target {
        self.0.as_raw()
    }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        let box_wait = unsafe { Raw::from_raw(raw) };
        Self(box_wait)
    }
}

pub struct TimeoutLock { tail: Atomic<RecursiveAcquire> }
type TimeoutGuard = ReleasePtr<Option<RecursiveAcquire>>;

impl TimeoutLock {
    pub fn try_acquire(&self, timeout: Duration) -> Option<TimeoutGuard> {
        let start = Instant::now();
        let (acquire, mut release) = AcquireBox::default();
        let mut acquire = self.tail.swap(RecursiveAcquire(acquire), Relaxed);
        while let Some(inner_acquire) = acquire.try_recur() {
            if start.elapsed() >= timeout {
                *release = Some(inner_acquire);
                return None;
            }
            acquire = inner_acquire;
        }
        Some(release)
    }
}

impl Lock for TimeoutLock {
    type Ref<'a> = &'a TimeoutLock;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for TimeoutLock {
    fn new() -> Self {
        let acquire = RecursiveAcquire(AcquireBox::default_acquired());
        TimeoutLock { tail: Atomic::new(acquire) }
    }
}

impl<'a> LockRef<'a> for &'a TimeoutLock {
    type Guard = TimeoutGuard;
    fn acquire(&mut self) -> Self::Guard {
        let (acquire, release) = AcquireBox::default();
        self.tail.swap(RecursiveAcquire(acquire), Relaxed);
        release
    }
}
