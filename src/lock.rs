use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};
use std::time::Duration;

use crate::atomic::{Atomic, Atomizable};
use crate::notify::{Notify, Wait};
use crate::Str;
use crate::backoff::Backoff;

pub trait Lock: Sized {
    type Ref<'a>: LockRef<'a> where Self: 'a;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str>;
}

pub trait BoundedLock: Lock {
    fn with_capacity(max_threads: usize) -> Result<Self, Str>;
    fn capacity(&self) -> usize;
    fn refs_left(&self) -> usize;
}

pub trait UnboundedLock: Lock {
    fn new() -> Self;
}

pub trait LockRef<'a>: Send {
    // the guard's drop method should release the lock
    type Guard: Drop;
    fn acquire(&mut self) -> Self::Guard;
}


pub struct PetersonLock {
    flags: [AtomicBool; 2],
    victim: AtomicBool,
    refs_left: u8,
}
pub struct PetersonLockRef<'a> {
    lock: &'a PetersonLock,
    id: bool,
}
pub struct FlagGuard<'a> { flag: &'a AtomicBool }

impl Lock for PetersonLock {
    type Ref<'a> = PetersonLockRef<'a>;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left > 0 {
            self.refs_left -= 1;
            Ok(PetersonLockRef {
                lock: self,
                id: self.refs_left != 0,
            })
        } else { Err("thread capacity exceeded") }
    }
}

impl BoundedLock for PetersonLock {
    fn with_capacity(max_threads: usize) -> Result<Self, Str> {
        if max_threads > 2 {
            Err("Peterson lock cannot support more than two threads")
        } else {
            Ok(PetersonLock {
                flags: [AtomicBool::new(false), AtomicBool::new(false)],
                victim: AtomicBool::new(false),
                refs_left: 2,
            })
        }
    }
    fn capacity(&self) -> usize { 2 }
    fn refs_left(&self) -> usize { self.refs_left as usize }
}

impl<'a> LockRef<'a> for PetersonLockRef<'a> {
    type Guard = FlagGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let PetersonLock { flags, victim, refs_left: _ } = self.lock;
        let my_flag = if self.id { &flags[1] } else { &flags[0] };
        let other_flag = if self.id { &flags[0] } else { &flags[1] };
        my_flag.store(true, Release);
        victim.store(self.id, Release);
        while other_flag.load(Acquire) &&
            victim.load(Acquire) == self.id {}
        FlagGuard { flag: my_flag }
    }
}

impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.flag.store(false, Release);
    }
}


pub struct FilterLock {
    levels: Box<[AtomicUsize]>,
    victims: Box<[AtomicUsize]>,
    refs_left: usize,
}
pub struct FilterLockRef<'a> {
    lock: &'a FilterLock,
    id: usize,
}
pub struct LevelGuard<'a> { level: &'a AtomicUsize }

impl Lock for FilterLock {
    type Ref<'a> = FilterLockRef<'a>;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left > 0 {
            self.refs_left -= 1;
            Ok(FilterLockRef { lock: self, id: self.refs_left })
        } else { Err("thread capacity exceeded") }
    }
}

impl BoundedLock for FilterLock {
    fn with_capacity(max_threads: usize) -> Result<Self, Str> {
        let mut levels: Vec<AtomicUsize> = Vec::with_capacity(max_threads);
        let mut victims: Vec<AtomicUsize> = Vec::with_capacity(max_threads);
        for _ in 0..max_threads {
            levels.push(AtomicUsize::new(0));
            victims.push(AtomicUsize::new(0));
        }
        Ok(FilterLock {
            levels: levels.into_boxed_slice(),
            victims: victims.into_boxed_slice(),
            refs_left: max_threads,
        })
    }
    fn capacity(&self) -> usize { self.levels.len() }
    fn refs_left(&self) -> usize { self.refs_left }
}

impl<'a> LockRef<'a> for FilterLockRef<'a> {
    type Guard = LevelGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let FilterLock { levels, victims, refs_left: _ } = self.lock;
        let capacity = self.lock.capacity();
        for i in 1..capacity {
            levels[self.id].store(i, Release);
            victims[i].store(self.id, Release);
            // spin until no other threads are ahead
            while (0..capacity).any(|k| {
                if k == self.id { return false }
                if levels[k].load(Acquire) < i { return false }
                victims[i].load(Acquire) == self.id
            }) {}
        }
        LevelGuard { level: &levels[self.id] }
    }
}

impl Drop for LevelGuard<'_> {
    fn drop(&mut self) {
        self.level.store(0, Release);
    }
}


pub struct BakeryLock {
    flags: Box<[AtomicBool]>,
    labels: Box<[AtomicUsize]>,
    refs_left: usize,
}
pub struct BakeryLockRef<'a> {
    lock: &'a BakeryLock,
    id: usize,
}

impl Lock for BakeryLock {
    type Ref<'a> = BakeryLockRef<'a>;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left > 0 {
            self.refs_left -= 1;
            Ok(BakeryLockRef { lock: self, id: self.refs_left })
        } else { Err("thread capacity exceeded") }
    }
}

impl BoundedLock for BakeryLock {
    fn with_capacity(max_threads: usize) -> Result<Self, Str> {
        let mut flags: Vec<AtomicBool> = Vec::with_capacity(max_threads);
        let mut labels: Vec<AtomicUsize> = Vec::with_capacity(max_threads);
        for _ in 0..max_threads {
            flags.push(AtomicBool::new(false));
            labels.push(AtomicUsize::new(0));
        }
        Ok(BakeryLock {
            flags: flags.into_boxed_slice(),
            labels: labels.into_boxed_slice(),
            refs_left: max_threads,
        })
    }
    fn capacity(&self) -> usize { self.flags.len() }
    fn refs_left(&self) -> usize { self.refs_left }
}

impl<'a> LockRef<'a> for BakeryLockRef<'a> {
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
        FlagGuard { flag: &flags[self.id] }
    }
}


pub struct TasLock { locked: AtomicBool }
pub struct TasGuard<'a> { locked: &'a AtomicBool }

impl Lock for TasLock {
    type Ref<'a> = &'a TasLock;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for TasLock {
    fn new() -> Self {
        TasLock { locked: AtomicBool::new(false) }
    }
}

impl<'a> LockRef<'a> for &'a TasLock {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let locked = &self.locked;
        while locked.swap(true, Acquire) {};
        TasGuard { locked }
    }
}

impl Drop for TasGuard<'_> {
    fn drop(&mut self) {
        self.locked.store(false, Release);
    }
}


pub struct TtasLock { locked: AtomicBool }

impl TtasLock {
    fn try_lock(&self) -> bool {
        while self.locked.load(Acquire) {};
        !self.locked.swap(true, Acquire)
    }
}

impl Lock for TtasLock {
    type Ref<'a> = &'a TtasLock;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for TtasLock {
    fn new() -> Self {
        TtasLock { locked: AtomicBool::new(false) }
    }
}

impl<'a> LockRef<'a> for &'a TtasLock {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        while !self.try_lock() {};
        TasGuard { locked: &self.locked }
    }
}


pub struct BackoffLock {
    ttas: TtasLock,
    min_delay: Duration,
    max_delay: Duration,
}

impl Lock for BackoffLock {
    type Ref<'a> = &'a BackoffLock;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for BackoffLock {
    fn new() -> Self {
        BackoffLock {
            ttas: TtasLock::new(),
            min_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1000),
        }
    }
}

impl<'a> LockRef<'a> for &'a BackoffLock {
    type Guard = TasGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let mut backoff = Backoff::new(self.min_delay, self.max_delay);
        while !self.ttas.try_lock() { backoff.backoff(); }
        TasGuard { locked: &self.ttas.locked }
    }
}


pub struct ArrayLock {
    flags: Box<[AtomicBool]>,
    next_slot: AtomicUsize,
    refs_left: usize,
}
pub struct ArrayLockRef<'a>(&'a ArrayLock);
pub struct ArrayGuard<'a> {
    curr_flag: &'a AtomicBool,
    next_flag: &'a AtomicBool,
}

impl ArrayLock {
    fn get_flag(&self, slot: usize) -> &AtomicBool {
        // index is always in bounds because of the modulo
        unsafe { &self.flags.get_unchecked(slot % self.capacity()) }
    }
}

impl Lock for ArrayLock {
    type Ref<'a> = ArrayLockRef<'a>;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left > 0 {
            self.refs_left -= 1;
            Ok(ArrayLockRef(self))
        } else { Err("thread capacity exceeded") }
    }
}

impl BoundedLock for ArrayLock {
    fn with_capacity(max_threads: usize) -> Result<Self, Str> {
        let mut flags: Vec<AtomicBool> = Vec::with_capacity(max_threads);
        flags.push(AtomicBool::new(true));
        for _ in 1..max_threads { flags.push(AtomicBool::new(false)); }
        Ok(ArrayLock {
            flags: flags.into_boxed_slice(),
            next_slot: AtomicUsize::new(0),
            refs_left: max_threads,
        })
    }
    fn capacity(&self) -> usize { self.flags.len() }
    fn refs_left(&self) -> usize { self.refs_left }
}

impl<'a> LockRef<'a> for ArrayLockRef<'a> {
    type Guard = ArrayGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let lock = self.0;
        // using AcqRel ensures fairness
        let slot = lock.next_slot.fetch_add(1, AcqRel);
        let curr_flag = lock.get_flag(slot);
        let next_flag = lock.get_flag(slot + 1);
        while !curr_flag.load(Acquire) {};
        ArrayGuard { curr_flag, next_flag }
    }
}

impl Drop for ArrayGuard<'_> {
    fn drop(&mut self) {
        self.curr_flag.store(false, Release);
        self.next_flag.store(true, Release);
    }
}


pub struct CLHLock { tail: Atomic<Box<Wait<()>>> }

impl Lock for CLHLock {
    type Ref<'a> = &'a CLHLock;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for CLHLock {
    fn new() -> Self {
        CLHLock { tail: Atomic::new(Wait::already_notified()) }
    }
}

impl<'a> LockRef<'a> for &'a CLHLock {
    type Guard = Notify<()>;
    fn acquire(&mut self) -> Self::Guard {
        let (wait, notify) = Wait::new();
        self.tail.swap(wait);
        notify
    }
}


type AtomicNotify<T> = Atomic<Option<Notify<T>>>;
pub struct MCSLock { tail: AtomicNotify<AtomicNotify<()>> }
pub struct MCSGuard<'a> {
    tail: &'a AtomicNotify<AtomicNotify<()>>,
    wait: Box<Wait<AtomicNotify<()>>>,
}

impl Lock for MCSLock {
    type Ref<'a> = &'a MCSLock;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        Ok(self)
    }
}

impl UnboundedLock for MCSLock {
    fn new() -> Self {
        MCSLock { tail: Atomic::new(None) }
    }
}

impl<'a> LockRef<'a> for &'a MCSLock {
    type Guard = MCSGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let (wait, notify) = Wait::with(Atomic::new(None));
        if let Some(notify) = self.tail.swap(Some(notify)) {
            let (inner_wait, inner_notify) = Wait::new();
            notify.swap(Some(inner_notify));
            drop(notify);
            inner_wait.wait();
        }
        MCSGuard { tail: &self.tail, wait }
    }
}

impl<'a> Drop for MCSGuard<'a> {
    fn drop(&mut self) {
        let notify_raw = self.wait.as_raw();
        drop(self.tail.compare_swap(notify_raw, None));
        drop(self.wait.wait().take());
    }
}
