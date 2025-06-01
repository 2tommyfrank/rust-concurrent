use std::sync::atomic::{AtomicUsize, Ordering::*};

use crate::guard::LevelGuard;
use crate::Str;

use super::{BoundedLock, Lock, LockRef};

pub struct FilterLock {
    levels: Box<[AtomicUsize]>,
    victims: Box<[AtomicUsize]>,
    refs_left: usize,
}

pub struct FilterRef<'a> {
    lock: &'a FilterLock,
    id: usize,
}

impl Lock for FilterLock {
    type Ref<'a> = FilterRef<'a>;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left > 0 {
            self.refs_left -= 1;
            Ok(FilterRef { lock: self, id: self.refs_left })
        } else { Err("thread capacity exceeded") }
    }
}

impl BoundedLock for FilterLock {
    fn with_capacity(max_threads: usize) -> Self {
        let mut levels: Vec<AtomicUsize> = Vec::with_capacity(max_threads);
        let mut victims: Vec<AtomicUsize> = Vec::with_capacity(max_threads);
        for _ in 0..max_threads {
            levels.push(AtomicUsize::new(0));
            victims.push(AtomicUsize::new(0));
        }
        FilterLock {
            levels: levels.into_boxed_slice(),
            victims: victims.into_boxed_slice(),
            refs_left: max_threads,
        }
    }
    fn capacity(&self) -> usize { self.levels.len() }
    fn refs_left(&self) -> usize { self.refs_left }
}

impl<'a> LockRef<'a> for FilterRef<'a> {
    type Guard = LevelGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let FilterLock { levels, victims, refs_left: _ } = self.lock;
        let capacity = self.lock.capacity();
        for i in 1..capacity {
            // Similar to Peterson lock: spin until no other threads are ahead
            levels[self.id].store(i, Relaxed);
            victims[i].swap(self.id, AcqRel);
            while (0..capacity).any(|k| {
                if k == self.id { return false }
                if levels[k].load(Acquire) < i { return false }
                victims[i].load(Relaxed) == self.id
            }) { }
        }
        LevelGuard::new(&levels[self.id])
    }
}
