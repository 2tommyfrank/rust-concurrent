use std::{ptr::NonNull, sync::atomic::{AtomicBool, Ordering}};

pub struct Wait(AtomicBool);
pub struct Notify(NonNull<AtomicBool>);

impl Wait {
    pub fn new() -> (Box<Self>, Notify) {
        let flag = AtomicBool::new(false);
        let notify = Notify(NonNull::from(&flag));
        let wait = Box::new(Wait(flag));
        (wait, notify)
    }
    pub fn already_notified() -> Box<Self> {
        Box::new(Wait(AtomicBool::new(true)))
    }
}

impl Drop for Wait {
    fn drop(&mut self) {
        while self.0.load(Ordering::Acquire) {}
    }
}

impl Drop for Notify {
    fn drop(&mut self) {
        let wait = unsafe { self.0.as_ref() };
        wait.store(true, Ordering::Release);
    }
}
