use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize};
use std::time::Duration;

use crate::backoff::Backoff;

pub trait Lock: Sized + Sync {
    type Guard<'a> where Self: 'a;
    fn acquire(&self) -> Self::Guard<'_>;
}

pub struct TASLock { locked: AtomicBool }
pub struct TASGuard<'a> { lock: &'a TASLock }

impl TASLock {
    pub fn new() -> Self {
        TASLock { locked: AtomicBool::new(false) }
    }
}

impl Lock for TASLock {
    type Guard<'a> = TASGuard<'a> where Self: 'a;
    fn acquire(&self) -> Self::Guard<'_> {
        while self.locked.swap(true, Ordering::Acquire) {};
        TASGuard { lock: &self }
    }
}

impl Drop for TASGuard<'_> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

pub struct TTASLock(TASLock);

impl TTASLock {
    pub fn new() -> Self { TTASLock(TASLock::new()) }
    fn try_lock(&self) -> bool {
        while self.0.locked.load(Ordering::Relaxed) {};
        !self.0.locked.swap(true, Ordering::Acquire)
    }
}

impl Lock for TTASLock {
    type Guard<'a> = TASGuard<'a> where Self: 'a;
    fn acquire(&self) -> Self::Guard<'_> {
        while !self.try_lock() {};
        TASGuard { lock: &self.0 }
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
            max_delay: Duration::from_millis(1000),
        }
    }
}

impl Lock for BackoffLock {
    type Guard<'a> = TASGuard<'a> where Self: 'a;
    fn acquire(&self) -> Self::Guard<'_> {
        let mut backoff = Backoff::new(self.min_delay, self.max_delay);
        while !self.ttas.try_lock() { backoff.backoff(); }
        TASGuard { lock: &self.ttas.0 }
    }
}

pub struct ArrayLock {
    flags: Box<[AtomicBool]>,
    next_slot: AtomicUsize,
    guards_left: AtomicUsize,
}

pub struct ArrayGuard<'a> {
    lock: &'a ArrayLock,
    slot: usize,
}

impl ArrayLock {
    // ArrayLock is only designed to work with a set finite number of threads
    pub fn new(max_threads: usize) -> Self {
        let mut flags: Vec<AtomicBool> = Vec::with_capacity(max_threads);
        flags.push(AtomicBool::new(true));
        for _ in 1..max_threads { flags.push(AtomicBool::new(false)); }
        ArrayLock {
            flags: flags.into_boxed_slice(),
            next_slot: AtomicUsize::new(0),
            guards_left: AtomicUsize::new(max_threads),
        }
    }
    pub fn capacity(&self) -> usize { self.flags.len() }
    fn get_flag(&self, slot: usize) -> &AtomicBool {
        // index is always in bounds because of the modulo
        unsafe { &self.flags.get_unchecked(slot % self.capacity()) }
    }
}

impl Lock for ArrayLock {
    type Guard<'a> = ArrayGuard<'a> where Self: 'a;
    fn acquire(&self) -> Self::Guard<'_> {
        if self.guards_left.fetch_sub(1, Ordering::Acquire) == 0 {
            self.guards_left.fetch_add(1, Ordering::Release);
            panic!("too many threads trying to acquire ArrayLock");
        }
        // AcqRel on fetch_add ensures fairness
        let slot = self.next_slot.fetch_add(1, Ordering::AcqRel);
        while !self.get_flag(slot).load(Ordering::Acquire) {};
        ArrayGuard { lock: self, slot }
    }
}

impl Drop for ArrayGuard<'_> {
    fn drop(&mut self) {
        self.lock.get_flag(self.slot).store(false, Ordering::Release);
        // now self.slot is safe to be used by another thread
        self.lock.guards_left.fetch_add(1, Ordering::Release);
        self.lock.get_flag(self.slot + 1).store(true, Ordering::Release);
    }
}
