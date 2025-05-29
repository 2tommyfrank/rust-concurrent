use std::mem;
use std::ptr::{null_mut, NonNull};
use std::sync::Arc;

use crate::atomic::Atomizable;

pub trait Raw: Sized {
    type Target: Atomizable;
    fn as_raw(&self) -> Self::Target;
    unsafe fn from_raw(raw: Self::Target) -> Self;

    fn into_raw(self) -> Self::Target {
        let raw = self.as_raw();
        mem::forget(self);
        raw
    }
}

impl<T> Raw for NonNull<T> {
    type Target = *mut T;
    fn as_raw(&self) -> Self::Target { self.as_ptr() }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        unsafe { NonNull::new_unchecked(raw) }
    }
}

impl<T> Raw for Box<T> {
    type Target = NonNull<T>;
    fn as_raw(&self) -> Self::Target { NonNull::from(self.as_ref()) }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        unsafe { Box::from_raw(raw.as_ptr()) }
    }
}

impl<T> Raw for Arc<T> {
    type Target = NonNull<T>;
    fn as_raw(&self) -> Self::Target { NonNull::from(self.as_ref()) }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        unsafe { Arc::from_raw(raw.as_ptr()) }
    }
}

impl<R, T: Raw<Target = NonNull<R>>> Raw for Option<T> {
    type Target = *mut R;
    fn as_raw(&self) -> Self::Target {
        match self {
            None => null_mut(),
            Some(t) => t.as_raw().as_ptr(),
        }
    }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        NonNull::new(raw).map(|raw| unsafe { T::from_raw(raw) })
    }
}
