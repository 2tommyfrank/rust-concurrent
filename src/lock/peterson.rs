use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering::*};

use crate::guard::FlagGuard;
use crate::lock::BorrowError::{self, *};

use super::{BoundedLock, Lock, LockRef};

pub struct PetersonLock {
    flags: [AtomicBool; 2],
    victim: AtomicBool,
    refs_left: AtomicIsize,
}

pub struct PetersonRef<'a> {
    lock: &'a PetersonLock,
    id: bool,
}

impl Lock for PetersonLock {
    type Ref<'a> = PetersonRef<'a>;
    fn borrow(&self) -> Result<Self::Ref<'_>, BorrowError> {
        let refs_left = self.refs_left.fetch_sub(1, Relaxed);
        if refs_left > 0 {
            Ok(PetersonRef {
                lock: self,
                id: refs_left == 1,
            })
        } else {
            self.refs_left.fetch_add(1, Relaxed);
            Err(ThreadCapacityExceeded)
        }
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
                refs_left: AtomicIsize::new(2),
            }
        }
    }
    fn capacity(&self) -> usize { 2 }
    fn refs_left(&self) -> usize {
        let refs_left = self.refs_left.load(Relaxed);
        if refs_left < 0 { 0 } else { refs_left as usize }
    }
}

impl Drop for PetersonRef<'_> {
    fn drop(&mut self) {
        self.lock.refs_left.fetch_add(1, Relaxed);
    }
}

impl<'a> LockRef<'a> for PetersonRef<'a> {
    type Guard = FlagGuard<'a>;
    fn acquire(&mut self) -> Self::Guard {
        let PetersonLock { flags, victim, refs_left: _ } = self.lock;
        let my_flag = if self.id { &flags[1] } else { &flags[0] };
        let other_flag = if self.id { &flags[0] } else { &flags[1] };
        my_flag.store(true, Relaxed);
        // While the result of the swap is unused, an order needs to be
        // established between this thread setting my_flag and the other
        // thread setting other_flag. The AcqRel here accomplishes this.
        victim.swap(self.id, AcqRel);
        while other_flag.load(Acquire) && victim.load(Relaxed) == self.id { }
        FlagGuard::new(my_flag)
    }
}
