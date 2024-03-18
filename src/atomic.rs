use std::sync::atomic::{AtomicPtr, Ordering};

pub unsafe trait Raw {
    type Target;
    unsafe fn into_raw(self) -> *mut Self::Target;
    unsafe fn from_raw(raw: *mut Self::Target) -> Self;
}

unsafe impl<T> Raw for Box<T> {
    type Target = T;
    unsafe fn into_raw(self) -> *mut T { Box::into_raw(self) }
    unsafe fn from_raw(raw: *mut T) -> Self {
        unsafe { Box::from_raw(raw) }
}
}


pub struct Atomic<T: Raw>(AtomicPtr<T::Target>);

impl<T: Raw> Atomic<T> {
    pub fn new(t: T) -> Self {
        let raw_t = unsafe { t.into_raw() };
        Atomic(AtomicPtr::new(raw_t))
    }
    pub fn swap(&self, t: T, order: Ordering) -> T {
        let raw_t = unsafe { t.into_raw() };
        let swapped = self.0.swap(raw_t, order);
        unsafe { T::from_raw(swapped) }
    }
}

impl<T: Raw> Drop for Atomic<T> {
    fn drop(&mut self) {
        let raw_t = self.0.get_mut();
        unsafe { drop(T::from_raw(*raw_t)) }
    }
}
