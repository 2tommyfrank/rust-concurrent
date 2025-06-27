use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::raw::Raw;

pub struct Transferable<T> {
    flag: AtomicBool,
    t: T,
}
pub struct AcquireBox<T>(Box<Transferable<T>>);
pub struct ReleasePtr<T> {
    // NonNull enables space optimization of Option<ReleasePtr<T>>
    ptr: NonNull<Transferable<T>>,
    phantom: PhantomData<*mut T>, // invariant over T
}

impl<T> Transferable<T> {
    fn release(&self) { self.flag.store(true, Release); }
    fn acquire(&self) {
        while !self.flag.load(Acquire) { }
    }
    fn try_acquire(&self) -> bool { self.flag.load(Acquire) }
}

impl<T> AcquireBox<T> {
    pub fn new(t: T) -> (Self, ReleasePtr<T>) {
        let acquirable = Transferable { flag: AtomicBool::new(false), t };
        let acquire_box = Box::new(acquirable);
        let release_ptr = ReleasePtr {
            ptr: NonNull::from(acquire_box.as_ref()),
            phantom: PhantomData,
        };
        (AcquireBox(acquire_box), release_ptr)
    }
    pub fn acquired(t: T) -> Self {
        let acquirable = Transferable { flag: AtomicBool::new(true), t };
        AcquireBox(Box::new(acquirable))
    }
    pub fn try_as_ref(&self) -> Result<&T, ()> {
        if self.0.try_acquire() { Ok(&self.0.t) }
        else { Err(()) }
    }
    pub fn try_as_mut(&mut self) -> Result<&mut T, ()> {
        if self.0.try_acquire() { Ok(&mut self.0.t) }
        else { Err(()) }
    }
    pub fn reset(&mut self) -> ReleasePtr<T> {
        self.0.acquire();
        *self.0.flag.get_mut() = false;
        let notify = ReleasePtr {
            ptr: NonNull::from(self.0.as_ref()),
            phantom: PhantomData
        };
        notify
    }
}

impl<T> AsRef<T> for AcquireBox<T> {
    fn as_ref(&self) -> &T {
        self.0.acquire();
        &self.0.t
    }
}

impl<T> AsMut<T> for AcquireBox<T> {
    fn as_mut(&mut self) -> &mut T {
        self.0.acquire();
        &mut self.0.t
    }
}

impl<T> Drop for AcquireBox<T> {
    fn drop(&mut self) { self.0.acquire(); }
}

impl<T> Raw for AcquireBox<T> {
    type Target = NonNull<Transferable<T>>;
    fn as_raw(&self) -> Self::Target { self.0.as_raw() }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        let acquire_box = unsafe { Raw::from_raw(raw) };
        AcquireBox(acquire_box)
    }
}

impl<T: Default> AcquireBox<T> {
    pub fn default() -> (Self, ReleasePtr<T>) {
        Self::new(T::default())
    }
    pub fn default_acquired() -> Self {
        Self::acquired(T::default())
    }
}

impl<T> Deref for ReleasePtr<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().t }
    }
}

impl<T> DerefMut for ReleasePtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.ptr.as_mut().t }
    }
}

impl<T> Drop for ReleasePtr<T> {
    fn drop(&mut self) {
        let acquirable = unsafe { self.ptr.as_ref() };
        acquirable.release();
    }
}

impl<T> Raw for ReleasePtr<T> {
    type Target = NonNull<Transferable<T>>;
    fn as_raw(&self) -> Self::Target { self.ptr }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        ReleasePtr { ptr: raw, phantom: PhantomData }
    }
}

unsafe impl<T> Send for ReleasePtr<T> { }

pub struct RecursiveAcquire(AcquireBox<Option<RecursiveAcquire>>);

impl RecursiveAcquire {
    pub fn new(acquire: AcquireBox<Option<RecursiveAcquire>>) -> Self {
        Self(acquire)
    }
    pub fn try_recur(mut self) -> Option<Self> {
        match self.0.try_as_mut() {
            Ok(next) => next.take().and_then(Self::try_recur),
            Err(()) => Some(self),
        }
    }
}

impl Raw for RecursiveAcquire {
    type Target = NonNull<Transferable<Option<RecursiveAcquire>>>;
    fn as_raw(&self) -> Self::Target {
        self.0.as_raw()
    }
    unsafe fn from_raw(raw: Self::Target) -> Self {
        let box_wait = unsafe { Raw::from_raw(raw) };
        Self(box_wait)
    }
}
