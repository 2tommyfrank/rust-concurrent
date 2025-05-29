use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::guard::FlagGuard;
use crate::Str;

use super::{BoundedLock, Lock, LockRef};

pub struct PetersonLock {
    flags: [AtomicBool; 2],
    victim: AtomicBool,
    refs_left: u8,
}

pub struct PetersonRef<'a> {
    lock: &'a PetersonLock,
    id: bool,
}

impl Lock for PetersonLock {
    type Ref<'a> = PetersonRef<'a>;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str> {
        if self.refs_left > 0 {
            self.refs_left -= 1;
            Ok(PetersonRef {
                lock: self,
                id: self.refs_left != 0,
            })
        } else { Err("thread capacity exceeded") }
    }
}

impl BoundedLock for PetersonLock {
    fn with_capacity(max_threads: usize) -> Self {
        if max_threads > 2 {
            panic!("Peterson lock cannot support more than two threads")
        } else {
            PetersonLock {
                flags: [AtomicBool::new(false), AtomicBool::new(false)],
                victim: AtomicBool::new(false),
                refs_left: 2,
            }
        }
    }
    fn capacity(&self) -> usize { 2 }
    fn refs_left(&self) -> usize { self.refs_left as usize }
}

impl<'a> LockRef<'a> for PetersonRef<'a> {
    type Guard = FlagGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let PetersonLock { flags, victim, refs_left: _ } = self.lock;
        let my_flag = if self.id { &flags[1] } else { &flags[0] };
        let other_flag = if self.id { &flags[0] } else { &flags[1] };
        my_flag.store(true, Release);
        victim.store(self.id, Release);
        while other_flag.load(Acquire) &&
            victim.load(Acquire) == self.id {}
        FlagGuard::new(my_flag)
    }
}
