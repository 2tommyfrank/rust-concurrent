use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::*};

use crate::guard::FlagGuard;
use crate::Str;

use super::{BoundedLock, Lock, LockRef};

pub struct BakeryLock {
    flags: Box<[AtomicBool]>,
    labels: Box<[AtomicU64]>,
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
        let mut labels: Vec<AtomicU64> = Vec::with_capacity(max_threads);
        for _ in 0..max_threads {
            flags.push(AtomicBool::new(false));
            labels.push(AtomicU64::new(0));
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
        // An order needs to be established between threads setting their
        // flags. But a write to a flag should be observed if the corresponding
        // write to the label is *not* observed, which doesn't work well with
        // Release-Acquire semantics. Instead, all these operations have been
        // made sequentially consistent.
        flags[self.id].store(true, SeqCst);
        let mut max_label: u64 = 0;
        for label in labels.as_ref() {
            let label = label.load(SeqCst);
            if label > max_label { max_label = label; }
        }
        let my_label = max_label + 1;
        labels[self.id].store(my_label, SeqCst);
        while (0..capacity).any(|k| {
            if k == self.id { return false }
            if !flags[k].load(SeqCst) { return false }
            let other_label = labels[k].load(Relaxed);
            if other_label < my_label { return true }
            if other_label > my_label { return false }
            k < self.id
        }) { }
        FlagGuard::new(&flags[self.id])
    }
}
