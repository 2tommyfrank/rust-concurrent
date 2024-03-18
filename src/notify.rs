use std::{ptr::NonNull, sync::atomic::{AtomicBool, Ordering}};

pub struct Wait(AtomicBool);
pub struct Notify(NonNull<AtomicBool>);

impl Wait {
    pub fn new() -> (Box<Self>, Notify) {
        let wait = Box::new(Wait(AtomicBool::new(false)));
        let flag = &wait.as_ref().0;
        let notify = Notify(NonNull::from(flag));
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
        let flag = unsafe { self.0.as_ref() };
        flag.store(true, Ordering::Release);
    }
}
