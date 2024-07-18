use std::{ptr::null_mut, sync::atomic::{AtomicPtr, Ordering::{self, *}}};

pub trait Raw: Send {
    type Target;
    fn into_raw(self) -> *mut Self::Target;
    unsafe fn from_raw(raw: *mut Self::Target) -> Self;
}
pub unsafe trait RawNonNull: Raw {}

impl<T: Send> Raw for Box<T> {
    type Target = T;
    fn into_raw(self) -> *mut Self::Target { Box::into_raw(self) }
    unsafe fn from_raw(raw: *mut Self::Target) -> Self {
        unsafe { Box::from_raw(raw) }
    }
}

unsafe impl<T: Send> RawNonNull for Box<T> {}

impl<T: RawNonNull> Raw for Option<T> {
    type Target = T::Target;
    fn into_raw(self) -> *mut Self::Target {
        match self {
            None => null_mut(),
            Some(t) => t.into_raw(),
        }
    }
    unsafe fn from_raw(raw: *mut Self::Target) -> Self {
        if raw.is_null() { None }
        else {
            unsafe { Some(T::from_raw(raw)) }
        }
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
    // AtomicPtr::compare_exchange returns the previous value on both success
    // and failure. This method, on the other hand, returns `new` on failure
    // in order to avoid duplication of the previous value.
    pub fn compare_swap(&self, current: *mut T::Target, new: T,
        order: Ordering) -> T
    {
        let new_raw = new.into_raw();
        match self.0.compare_exchange(current, new_raw, order, Relaxed) {
            Ok(raw) => unsafe { T::from_raw(raw) },
            Err(_) => unsafe { T::from_raw(new_raw) },
        }
    }
}

impl<T: Raw> Drop for Atomic<T> {
    fn drop(&mut self) {
        let raw_t = *self.0.get_mut();
        unsafe { drop(T::from_raw(raw_t)) }
    }
}

impl<T: RawNonNull> Atomic<Option<T>> {
    pub fn take(&self, order: Ordering) -> Option<T> { self.swap(None, order) }
}
