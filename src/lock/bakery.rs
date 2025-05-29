use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};

use crate::guard::FlagGuard;
use crate::Str;

use super::{BoundedLock, Lock, LockRef};

pub struct BakeryLock {
    flags: Box<[AtomicBool]>,
    labels: Box<[AtomicUsize]>,
    refs_left: usize,
}

pub struct BakeryRef<'a> {
    lock: &'a BakeryLock,
    id: usize,
}

impl Lock for BakeryLock {
    type Ref<'a> = BakeryRef<'a>;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left > 0 {
            self.refs_left -= 1;
            Ok(BakeryRef { lock: self, id: self.refs_left })
        } else { Err("thread capacity exceeded") }
    }
}

impl BoundedLock for BakeryLock {
    fn with_capacity(max_threads: usize) -> Self {
        let mut flags: Vec<AtomicBool> = Vec::with_capacity(max_threads);
        let mut labels: Vec<AtomicUsize> = Vec::with_capacity(max_threads);
        for _ in 0..max_threads {
            flags.push(AtomicBool::new(false));
            labels.push(AtomicUsize::new(0));
        }
        BakeryLock {
            flags: flags.into_boxed_slice(),
            labels: labels.into_boxed_slice(),
            refs_left: max_threads,
        }
    }
    fn capacity(&self) -> usize { self.flags.len() }
    fn refs_left(&self) -> usize { self.refs_left }
}

impl<'a> LockRef<'a> for BakeryRef<'a> {
    type Guard = FlagGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let BakeryLock { flags, labels, refs_left: _ } = self.lock;
        let capacity = self.lock.capacity();
        flags[self.id].store(true, Release);
        let mut max_label: usize = 0;
        for label in labels.as_ref() {
            let label = label.load(Acquire);
            if label > max_label { max_label = label; }
        }
        let my_label: usize = max_label + 1;
        labels[self.id].store(my_label, Release);
        while (0..capacity).any(|k| {
            if k == self.id { return false }
            if !flags[k].load(Acquire) { return false }
            let other_label = labels[k].load(Acquire);
            if other_label < my_label { return true }
            if other_label > my_label { return false }
            k < self.id
        }) {}
        FlagGuard::new(&flags[self.id])
    }
}
