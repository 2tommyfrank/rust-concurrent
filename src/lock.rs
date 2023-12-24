use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::backoff::Backoff;

pub trait Lock: Sized {
    fn lock(&mut self);
    fn unlock(&mut self);
    fn acquire(&mut self) -> Guard<'_, Self> {
        self.lock();
        Guard(self)
    }
}

pub struct Guard<'a, L: Lock>(&'a mut L);

impl<L: Lock> Drop for Guard<'_, L> {
    fn drop(&mut self) {
        self.0.unlock();
    }
}

pub struct TASLock { locked: AtomicBool, }

impl TASLock {
    pub fn new() -> Self {
        TASLock { locked: AtomicBool::new(false) }
    }
}

impl Lock for TASLock {
    fn lock(&mut self) {
        while self.locked.swap(true, Ordering::Acquire) {}
    }
    fn unlock(&mut self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub struct TTASLock { locked: AtomicBool, }

impl TTASLock {
    pub fn new() -> Self {
        TTASLock { locked: AtomicBool::new(false) }
    }
    fn try_lock(&mut self) -> bool {
        while self.locked.load(Ordering::Relaxed) {}
        !self.locked.swap(true, Ordering::Acquire)
    }
}

impl Lock for TTASLock {
    fn lock(&mut self) {
        while !self.try_lock() {}
    }
    fn unlock(&mut self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub struct BackoffLock {
    ttas: TTASLock,
    min_delay: Duration,
    max_delay: Duration,
}

impl BackoffLock {
    pub fn new() -> Self {
        BackoffLock {
            ttas: TTASLock::new(),
            min_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1000)
        }
    }
}

impl Lock for BackoffLock {
    fn lock(&mut self) {
        let mut backoff = Backoff::new(self.min_delay, self.max_delay);
        while !self.ttas.try_lock() { backoff.backoff() }
    }
    fn unlock(&mut self) { self.ttas.unlock(); }
}
