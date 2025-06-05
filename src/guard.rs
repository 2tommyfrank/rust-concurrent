use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};

use crate::atomic::{Atomic, Atomizable};
use crate::notify::{Notify, Wait};

pub struct FlagGuard<'a> { flag: &'a AtomicBool }

impl<'a> FlagGuard<'a> {
    pub fn new(flag: &'a AtomicBool) -> Self {
        Self { flag }
    }
}

impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.flag.store(false, Release);
    }
}

pub struct LevelGuard<'a> { level: &'a AtomicUsize }

impl<'a> LevelGuard<'a> {
    pub fn new(level: &'a AtomicUsize) -> Self {
        Self { level }
    }
}

impl Drop for LevelGuard<'_> {
    fn drop(&mut self) {
        self.level.store(0, Release);
    }
}

pub struct TasGuard<'a> { locked: &'a AtomicBool }

impl<'a> TasGuard<'a> {
    pub fn new(locked: &'a AtomicBool) -> Self {
        Self { locked }
    }
}

impl Drop for TasGuard<'_> {
    fn drop(&mut self) {
        self.locked.store(false, Release);
    }
}

pub struct ArrayGuard<'a> {
    curr_flag: &'a AtomicBool,
    next_flag: &'a AtomicBool,
}

impl<'a> ArrayGuard<'a> {
    pub fn new(curr_flag: &'a AtomicBool, next_flag: &'a AtomicBool) -> Self {
        Self { curr_flag, next_flag }
    }
}

impl Drop for ArrayGuard<'_> {
    fn drop(&mut self) {
        self.curr_flag.store(false, Relaxed);
        self.next_flag.store(true, Release);
    }
}

pub struct McsGuard<'a> {
    tail: &'a Atomic<Option<Notify<Option<Notify<()>>>>>,
    wait: Box<Wait<Option<Notify<()>>>>,
}

impl<'a> McsGuard<'a> {
    pub fn new(tail: &'a Atomic<Option<Notify<Option<Notify<()>>>>>,
    wait: Box<Wait<Option<Notify<()>>>>) -> Self {
        Self { tail, wait }
    }
}

impl<'a> Drop for McsGuard<'a> {
    fn drop(&mut self) {
        let notify_raw = self.wait.as_raw();
        drop(self.tail.compare_swap_strong(notify_raw, None, Relaxed));
        self.wait.wait_mut().take();
    }
}
