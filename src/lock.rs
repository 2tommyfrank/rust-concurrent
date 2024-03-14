use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize, AtomicPtr};
use std::time::Duration;

use crate::Str;
use crate::backoff::Backoff;

pub trait Lock: Sized {
    type Ref<'a>: LockRef<'a> where Self: 'a;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str>;
}

pub trait LockRef<'a>: Send {
    // the guard's drop method should release the lock
    type Guard: Drop;
    fn acquire(&mut self) -> Self::Guard;
}

pub struct TasLock { locked: AtomicBool }
pub struct TasLockRef<'a>(&'a TasLock);
pub struct TasGuard<'a> { locked: &'a AtomicBool }

impl TasLock {
    pub fn new() -> Self {
        TasLock { locked: AtomicBool::new(false) }
    }
}

impl Lock for TasLock {
    type Ref<'a> = TasLockRef<'a>;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(TasLockRef(self))
    }
}

impl<'a> LockRef<'a> for TasLockRef<'a> {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let locked = &self.0.locked;
        while locked.swap(true, Ordering::Acquire) {};
        TasGuard { locked }
    }
}

impl Drop for TasGuard<'_> {
    fn drop(&mut self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub struct TtasLock { locked: AtomicBool }
pub struct TtasLockRef<'a>(&'a TtasLock);

impl TtasLock {
    pub fn new() -> Self {
        TtasLock { locked: AtomicBool::new(false) }
    }
    fn try_lock(&self) -> bool {
        while self.locked.load(Ordering::Acquire) {};
        !self.locked.swap(true, Ordering::Acquire)
    }
}

impl Lock for TtasLock {
    type Ref<'a> = TtasLockRef<'a>;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(TtasLockRef(self))
    }
}

impl<'a> LockRef<'a> for TtasLockRef<'a> {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let lock = self.0;
        while !lock.try_lock() {};
        TasGuard { locked: &lock.locked }
    }
}

pub struct BackoffLock {
    ttas: TtasLock,
    min_delay: Duration,
    max_delay: Duration,
}
pub struct BackoffLockRef<'a>(&'a BackoffLock);

impl BackoffLock {
    pub fn new() -> Self {
        BackoffLock {
            ttas: TtasLock::new(),
            min_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1000),
        }
    }
}

impl Lock for BackoffLock {
    type Ref<'a> = BackoffLockRef<'a>;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(BackoffLockRef(self))
    }
}

impl<'a> LockRef<'a> for BackoffLockRef<'a> {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let BackoffLock { ttas, min_delay, max_delay } = self.0;
        let mut backoff = Backoff::new(*min_delay, *max_delay);
        while !ttas.try_lock() { backoff.backoff(); }
        TasGuard { locked: &ttas.locked }
    }
}

pub struct ArrayLock {
    flags: Box<[AtomicBool]>,
    next_slot: AtomicUsize,
    // unsynchronized; should not be used by ArrayLockRef or ArrayGuard
    refs_left: Cell<usize>,
}
pub struct ArrayLockRef<'a>(&'a ArrayLock);
pub struct ArrayGuard<'a> {
    curr_flag: &'a AtomicBool,
    next_flag: &'a AtomicBool,
}

impl ArrayLock {
    // ArrayLock is only designed to work with a bounded number of threads
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
    pub fn capacity(&self) -> usize { self.flags.len() }
    pub fn refs_left(&self) -> usize { self.refs_left.get() }
    fn get_flag(&self, slot: usize) -> &AtomicBool {
        // index is always in bounds because of the modulo
        unsafe { &self.flags.get_unchecked(slot % self.capacity()) }
    }
}

impl Lock for ArrayLock {
    type Ref<'a> = ArrayLockRef<'a>;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left.get() > 0 {
            self.refs_left.update(|x| x - 1);
            Ok(ArrayLockRef(self))
        } else { Err("capacity exceeded") }
    }
}

impl<'a> LockRef<'a> for ArrayLockRef<'a> {
    type Guard = ArrayGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let lock = self.0;
        // using AcqRel ensures fairness
        let slot = lock.next_slot.fetch_add(1, Ordering::AcqRel);
        let curr_flag = lock.get_flag(slot);
        let next_flag = lock.get_flag(slot + 1);
        while !curr_flag.load(Ordering::Acquire) {};
        ArrayGuard { curr_flag, next_flag }
    }
}

unsafe impl Send for ArrayLockRef<'_> {}

impl Drop for ArrayGuard<'_> {
    fn drop(&mut self) {
        self.curr_flag.store(false, Ordering::Release);
        self.next_flag.store(true, Ordering::Release);
    }
}

pub struct CLHLock {
    tail: AtomicPtr<AtomicBool>,
}
pub struct CLHLockRef<'a> {
    lock: &'a CLHLock,
    curr_node: Arc<AtomicBool>,
    prev_node: Option<Arc<AtomicBool>>,
}
pub struct CLHGuard<'a> {
    lock_ref: &'a CLHLockRef<'a>,
}

impl CLHLock {
    pub fn new() -> Self {
        let locked = AtomicBool::new(false);
        let tail = Arc::into_raw(Arc::new(locked)).cast_mut();
        CLHLock { tail: AtomicPtr::new(tail) }
    }
}

impl Drop for CLHLock {
    fn drop(&mut self) {
        let tail: *const AtomicBool = *self.tail.get_mut();
        unsafe { drop(Arc::from_raw(tail)); }
    }
}

impl Lock for CLHLock {
    type Ref<'a> = CLHLockRef<'a>;
    fn borrow(&self) -> Result<Self::Ref<'_>, Str> {
        Ok(CLHLockRef {
            lock: self,
            curr_node: Arc::new(AtomicBool::new(false)),
            prev_node: None,
        })
    }
}

impl<'a> LockRef<'a> for CLHLockRef<'a> {
    type Guard = CLHGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let tail = &self.lock.tail;
        *self.curr_node.get_mut() = true;
        let curr_node = Arc::into_raw(self.curr_node).cast_mut();
        let prev_node = tail.swap(curr_node, Ordering::AcqRel);
        let prev_node = unsafe { Arc::from_raw(prev_node) };
        self.prev_node = Some(prev_node);
        while prev_node.load(Ordering::Acquire) {}
        CLHGuard { lock_ref: &self }
    }
}

impl Drop for CLHGuard<'_> {
    fn drop(&mut self) {
        let CLHLockRef { lock: _, mut curr_node, prev_node } = self.lock_ref;
        curr_node.store(false, Ordering::Release);
        curr_node = prev_node.take().expect("already unlocked");
    }
}
