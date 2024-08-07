use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::atomic::Raw;

pub struct Wait<T> {
    flag: AtomicBool,
    t: T,
}

impl<T> Wait<T> {
    pub fn with(t: T) -> (Box<Self>, Notify<T>) {
        let wait = Box::new(Wait { flag: AtomicBool::new(false), t });
        let notify = Notify(NonNull::from(wait.as_ref()));
        (wait, notify)
    }
    pub fn already_notified_with(t: T) -> Box<Self> {
        Box::new(Wait { flag: AtomicBool::new(true), t })
    }
    pub fn wait(&self) -> &T {
        while self.flag.load(Acquire) {}
        &self.t
    }
}

impl<T: Default> Wait<T> {
    pub fn new() -> (Box<Self>, Notify<T>) {
        Self::with(T::default())
    }
    pub fn already_notified() -> Box<Self> {
        Self::already_notified_with(T::default())
    }
}

impl<T> Drop for Wait<T> {
    fn drop(&mut self) { self.wait(); }
}


pub struct Notify<T>(NonNull<Wait<T>>);

impl<T> Deref for Notify<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.0.as_ref().t }
    }
}

impl<T> Drop for Notify<T> {
    fn drop(&mut self) {
        let wait = unsafe { self.0.as_ref() };
        wait.flag.store(true, Release);
    }
}

impl<T> Raw for Notify<T> {
    type Target = NonNull<Wait<T>>;
    fn as_raw(&self) -> Self::Target { self.0 }
    unsafe fn from_raw(raw: Self::Target) -> Self { Notify(raw) }
}

unsafe impl<T> Send for Notify<T> {}
