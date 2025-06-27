use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering::*};

use crate::guard::ArrayGuard;
use crate::lock::BorrowError::{self, *};

use super::{BoundedLock, Lock, LockRef};

pub struct ArrayLock {
    flags: Box<[AtomicBool]>,
    next_slot: AtomicUsize,
    refs_left: AtomicIsize,
}

pub struct ArrayRef<'a>(&'a ArrayLock);

impl ArrayLock {
    fn get_flag(&self, slot: usize) -> &AtomicBool {
        // index is always in bounds because of the modulo
        unsafe { &self.flags.get_unchecked(slot % self.capacity()) }
    }
}

impl Lock for ArrayLock {
    type Ref<'a> = ArrayRef<'a>;
    fn borrow(&self) -> Result<Self::Ref<'_>, BorrowError> {
        let refs_left = self.refs_left.fetch_sub(1, Relaxed);
        if refs_left > 0 {
            Ok(ArrayRef(self))
        } else {
            self.refs_left.fetch_add(1, Relaxed);
            Err(ThreadCapacityExceeded)
        }
    }
}

impl BoundedLock for ArrayLock {
    fn with_capacity(max_threads: usize) -> Self {
        let mut flags: Vec<AtomicBool> = Vec::with_capacity(max_threads);
        flags.push(AtomicBool::new(true));
        for _ in 1..max_threads { flags.push(AtomicBool::new(false)); }
        ArrayLock {
            flags: flags.into_boxed_slice(),
            next_slot: AtomicUsize::new(0),
            refs_left: AtomicIsize::new(max_threads as isize),
        }
    }
    fn capacity(&self) -> usize { self.flags.len() }
    fn refs_left(&self) -> usize {
        let refs_left = self.refs_left.load(Relaxed);
        if refs_left < 0 { 0 } else { refs_left as usize }
    }
}

impl Drop for ArrayRef<'_> {
    fn drop(&mut self) {
        self.0.refs_left.fetch_add(1, Relaxed);
    }
}

impl<'a> LockRef<'a> for ArrayRef<'a> {
    type Guard = ArrayGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let lock = self.0;
        let slot = lock.next_slot.fetch_add(1, Relaxed);
        let curr_flag = lock.get_flag(slot);
        let next_flag = lock.get_flag(slot + 1);
        while !curr_flag.load(Acquire) { };
        ArrayGuard::new(curr_flag, next_flag)
    }
}
