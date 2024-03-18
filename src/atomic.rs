use std::ffi::{c_char, CString};
use std::sync::{atomic::{AtomicPtr, Ordering}, Arc, Weak};

pub trait Raw: Send {
    type Target;
    fn into_raw(self) -> *mut Self::Target;
    unsafe fn from_raw(raw: *mut Self::Target) -> Self;
}

impl<T: Send> Raw for Box<T> {
    type Target = T;
    fn into_raw(self) -> *mut T { Box::into_raw(self) }
    unsafe fn from_raw(raw: *mut T) -> Self {
        unsafe { Box::from_raw(raw) }
    }
}

impl<T: Send + Sync> Raw for Arc<T> {
    type Target = T;
    fn into_raw(self) -> *mut Self::Target {
        Arc::into_raw(self).cast_mut()
    }
    unsafe fn from_raw(raw: *mut Self::Target) -> Self {
        unsafe { Arc::from_raw(raw) }
    }
}

impl<T: Send + Sync> Raw for Weak<T> {
    type Target = T;
    fn into_raw(self) -> *mut Self::Target {
        Weak::into_raw(self).cast_mut()
    }
    unsafe fn from_raw(raw: *mut Self::Target) -> Self {
        unsafe { Weak::from_raw(raw) }
    }
}

impl Raw for CString {
    type Target = c_char;
    fn into_raw(self) -> *mut Self::Target {
        CString::into_raw(self)
    }
    unsafe fn from_raw(raw: *mut Self::Target) -> Self {
        unsafe { CString::from_raw(raw) }
    }
}


pub struct Atomic<T: Raw>(AtomicPtr<T::Target>);

impl<T: Raw> Atomic<T> {
    pub fn new(t: T) -> Self {
        Atomic(AtomicPtr::new(t.into_raw()))
    }
    pub fn swap(&self, t: T, order: Ordering) -> T {
        let swapped = self.0.swap(t.into_raw(), order);
        unsafe { T::from_raw(swapped) }
    }
}

impl<T: Raw> Drop for Atomic<T> {
    fn drop(&mut self) {
        let raw_t = *self.0.get_mut();
        unsafe { drop(T::from_raw(raw_t)) }
    }
}
