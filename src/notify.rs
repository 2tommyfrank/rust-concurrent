use std::{ptr::NonNull, sync::atomic::{AtomicBool, Ordering}};

use crate::atomic::Raw;

pub struct Notify { flag: NonNull<AtomicBool> }
pub struct Wait { flag: Box<AtomicBool> }

impl Wait {
    pub fn new() -> (Self, Notify) {
        let flag = AtomicBool::new(false);
        let notify = Notify { flag: NonNull::from(&flag) };
        let wait = Wait { flag: Box::new(flag) };
        (wait, notify)
    }
    pub fn already_notified() -> Self {
        let flag = AtomicBool::new(true);
        Wait { flag: Box::new(flag) }
    }
}

impl Drop for Notify {
    fn drop(&mut self) {
        let flag = unsafe { self.flag.as_ref() };
        flag.store(true, Ordering::Release);
    }
}

impl Drop for Wait {
    fn drop(&mut self) {
        while self.flag.load(Ordering::Acquire) {}
    }
}

unsafe impl Raw for Wait {
    type Target = AtomicBool;
    unsafe fn into_raw(self) -> *mut Self::Target {
        let ptr: *const AtomicBool = self.flag.as_ref();
        std::mem::forget(self);
        ptr.cast_mut()
    }
    unsafe fn from_raw(raw: *mut Self::Target) -> Self {
        let flag = unsafe { Box::from_raw(raw) };
        Wait { flag }
    }
}
