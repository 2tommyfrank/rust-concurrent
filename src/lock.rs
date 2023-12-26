use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::backoff::Backoff;

pub trait Lock: Sized {
    fn new() -> Self;
    fn lock(&self);
    fn unlock(&self);
    fn acquire(&self) -> Guard<'_, Self> {
        self.lock();
        Guard(self)
    }
}

pub struct Guard<'a, L: Lock>(&'a L);

impl<L: Lock> Drop for Guard<'_, L> {
    fn drop(&mut self) {
        self.0.unlock();
    }
}

pub struct TASLock { locked: AtomicBool, }

impl Lock for TASLock {
    fn new() -> Self {
        TASLock { locked: AtomicBool::new(false) }
    }
    fn lock(&self) {
        while self.locked.swap(true, Ordering::Acquire) {}
    }
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub struct TTASLock { locked: AtomicBool, }

impl TTASLock {
    fn try_lock(&self) -> bool {
        while self.locked.load(Ordering::Relaxed) {}
        !self.locked.swap(true, Ordering::Acquire)
    }
}

impl Lock for TTASLock {
    fn new() -> Self {
        TTASLock { locked: AtomicBool::new(false) }
    }
    fn lock(&self) {
        while !self.try_lock() {}
    }
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub struct BackoffLock {
    ttas: TTASLock,
    min_delay: Duration,
    max_delay: Duration,
}

impl Lock for BackoffLock {
    fn new() -> Self {
        BackoffLock {
            ttas: TTASLock::new(),
            min_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1000)
        }
    }
    fn lock(&self) {
        let mut backoff = Backoff::new(self.min_delay, self.max_delay);
        while !self.ttas.try_lock() { backoff.backoff() }
    }
    fn unlock(&self) { self.ttas.unlock(); }
}
