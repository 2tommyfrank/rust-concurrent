use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::raw::Raw;

pub struct Wait<T> {
    flag: AtomicBool,
    t: T,
}

impl<T> Wait<T> {
    pub fn with(t: T) -> (Box<Self>, Notify<T>) {
        let wait = Box::new(Wait { flag: AtomicBool::new(false), t });
        let notify = Notify {
            ptr: NonNull::from(wait.as_ref()),
            phantom: PhantomData
        };
        (wait, notify)
    }
    pub fn already_notified_with(t: T) -> Box<Self> {
        Box::new(Wait { flag: AtomicBool::new(true), t })
    }
    pub fn wait(&self) -> &T {
        while !self.flag.load(Acquire) { }
        &self.t
    }
    pub fn wait_mut(&mut self) -> &mut T {
        while !self.flag.load(Acquire) { }
        &mut self.t
    }
    pub fn try_wait(&self) -> Result<&T, ()> {
        if self.flag.load(Acquire) { Ok(&self.t) }
        else { Err(()) }
    }
    pub fn try_wait_mut(&mut self) -> Result<&mut T, ()> {
        if self.flag.load(Acquire) { Ok(&mut self.t) }
        else { Err(()) }
    }
    pub fn wait_reset(self: &mut Box<Self>) -> Notify<T> {
        while !self.flag.load(Acquire) { }
        *self.flag.get_mut() = false;
        let notify = Notify {
            ptr: NonNull::from(self.as_ref()),
            phantom: PhantomData
        };
        notify
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


pub struct Notify<T> {
    ptr: NonNull<Wait<T>>,
    phantom: PhantomData<*mut T>, // invariant over T
}

impl<T> Deref for Notify<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().t }
    }
}

impl<T> DerefMut for Notify<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.ptr.as_mut().t }
    }
}

impl<T> Drop for Notify<T> {
    fn drop(&mut self) {
        let wait = unsafe { self.ptr.as_ref() };
        wait.flag.store(true, Release);
    }
}

impl<T> Raw for Notify<T> {
    type Target = NonNull<Wait<T>>;
    fn as_raw(&self) -> Self::Target { self.ptr }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        Notify { ptr: raw, phantom: PhantomData }
    }
}

unsafe impl<T> Send for Notify<T> { }
