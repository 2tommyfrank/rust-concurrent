use std::cell::Cell;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize};
use std::time::Duration;

use crate::backoff::Backoff;

pub trait Lock: Sized + Sync {
    fn new() -> Self;
    fn lock(&self);
    fn unlock(&self);
    fn acquire(&self) -> Guard<Self> {
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

pub struct TASLock { locked: AtomicBool }

impl Lock for TASLock {
    fn new() -> Self {
        TASLock { locked: AtomicBool::new(false) }
    }
    fn lock(&self) {
        while self.locked.swap(true, Ordering::Acquire) {};
    }
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub struct TTASLock { locked: AtomicBool }

impl TTASLock {
    fn try_lock(&self) -> bool {
        while self.locked.load(Ordering::Relaxed) {};
        !self.locked.swap(true, Ordering::Acquire)
    }
}

impl Lock for TTASLock {
    fn new() -> Self {
        TTASLock { locked: AtomicBool::new(false) }
    }
    fn lock(&self) {
        while !self.try_lock() {};
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
            max_delay: Duration::from_millis(1000),
        }
    }
    fn lock(&self) {
        let mut backoff = Backoff::new(self.min_delay, self.max_delay);
        while !self.ttas.try_lock() { backoff.backoff(); }
    }
    fn unlock(&self) { self.ttas.unlock(); }
}

pub struct ArrayLock {
    flags: Box<[AtomicBool]>,
    next_slot: AtomicUsize,
    refs_left: Cell<usize>,
}

pub struct ArrayLockRef<'a> {
    deref: &'a ArrayLock,
    slot: usize,
}

impl ArrayLock {
    pub fn new(max_threads: usize) -> Self {
        let mut flags: Vec<AtomicBool> = Vec::with_capacity(max_threads);
        flags.push(AtomicBool::new(true));
        for _ in 1..max_threads { flags.push(AtomicBool::new(false)); }
        ArrayLock {
            flags: flags.into_boxed_slice(),
            next_slot: AtomicUsize::new(0),
            refs_left: Cell::new(max_threads),
        }
    }
    pub fn borrow(&self) -> Option<ArrayLockRef> {
        if self.refs_left.get() == 0 { None }
        else {
            self.refs_left.update(|x| x - 1);
            Some(ArrayLockRef { deref: &self, slot: 0 })
        }
    }
    fn capacity(&self) -> usize { self.flags.len() }
    fn get_flag(&self, slot: usize) -> &AtomicBool {
        // index is always in bounds because of the modulo
        unsafe { &self.flags.get_unchecked(slot % self.capacity()) }
    }
}

impl ArrayLockRef<'_> {
    pub fn lock(&mut self) {
        // AcqRel on fetch_add ensures fairness
        self.slot = self.next_slot.fetch_add(1, Ordering::AcqRel);
        while !self.get_flag(self.slot).load(Ordering::Acquire) {};
    }
    pub fn unlock(&mut self) {
        self.get_flag(self.slot).store(false, Ordering::Release);
        self.get_flag(self.slot + 1).store(true, Ordering::Release);
    }
}

impl Deref for ArrayLockRef<'_> {
    type Target = ArrayLock;
    fn deref(&self) -> &Self::Target { &self.deref }
}

unsafe impl Send for ArrayLockRef<'_> {}
