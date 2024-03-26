use std::{ptr::NonNull, sync::atomic::{AtomicBool, Ordering}};

use crate::atomic::Raw;

pub struct Notify(NonNull<WaitFlag>);
pub struct WaitFlag(AtomicBool);

impl Drop for Notify {
    fn drop(&mut self) {
        unsafe {
            let flag = self.0.as_ref();
            flag.0.store(true, Ordering::Release);
        }
    }
}

impl Drop for WaitFlag {
    fn drop(&mut self) {
        while self.0.load(Ordering::Acquire) {}
    }
}

pub struct Wait(Box<WaitFlag>);

impl Wait {
    pub fn new() -> (Self, Notify) {
        let flag = Box::new(WaitFlag(AtomicBool::new(false)));
        let notify = Notify(NonNull::from(flag.as_ref()));
        let wait = Wait(flag);
        (wait, notify)
    }
    pub fn already_notified() -> Self {
        let flag = Box::new(WaitFlag(AtomicBool::new(true)));
        Wait(flag)
    }
}

impl Raw for Wait {
    type Target = WaitFlag;
    fn into_raw(self) -> *mut Self::Target { self.0.into_raw() }
    unsafe fn from_raw(raw: *mut Self::Target) -> Self {
        let flag = unsafe { Box::from_raw(raw) };
        Wait(flag)
    }
}
