use std::mem;
use std::sync::atomic::*;
use std::sync::atomic::Ordering::*;

use crate::raw::Raw;

pub trait Atomizable: Sized {
    type Atomic;
    type Raw;
    fn as_raw(&self) -> Self::Raw;
    fn into_atomic(self) -> Self::Atomic;
    // Calling load_atomic and/or get_atomic multiple times may violate the
    // borrow rules for Self.
    unsafe fn load_atomic(atomic: &Self::Atomic, order: Ordering) -> Self;
    fn store_atomic(self, atomic: &Self::Atomic, order: Ordering);
    fn swap_atomic(self, atomic: &Self::Atomic, order: Ordering) -> Self;
    // Atomic*::compare_exchange and compare_exchange_weak return the previous
    // value on both success and failure. These methods, on the other hand,
    // return self on failure in order to avoid duplication of the old value.
    fn compare_swap_strong(
        self, atomic: &Self::Atomic, compare: Self::Raw, order: Ordering
    ) -> Result<Self, Self>;
    fn compare_swap_weak(
        self, atomic: &Self::Atomic, compare: Self::Raw, order: Ordering
    ) -> Result<Self, Self>;
    unsafe fn get_atomic(atomic: &mut Self::Atomic) -> Self;
}

macro_rules! impl_atomizable {
    ($atomic:ty) => {
        type Atomic = $atomic;
        type Raw = Self;
        fn as_raw(&self) -> Self::Raw { *self }
        fn into_atomic(self) -> Self::Atomic { Self::Atomic::new(self) }
        unsafe fn load_atomic(atomic: &Self::Atomic, order: Ordering) -> Self {
            atomic.load(order)
        }
        fn store_atomic(self, atomic: &Self::Atomic, order: Ordering) {
            atomic.store(self, order)
        }
        fn swap_atomic(self, atomic: &Self::Atomic, order: Ordering) -> Self {
            atomic.swap(self, order)
        }
        fn compare_swap_strong(
            self, atomic: &Self::Atomic, compare: Self::Raw, order: Ordering
        ) -> Result<Self, Self> {
            match atomic.compare_exchange(compare, self, order, Relaxed) {
                Ok(old) => Ok(old),
                Err(_) => Err(self),
            }
        }
        fn compare_swap_weak(
            self, atomic: &Self::Atomic, compare: Self::Raw, order: Ordering
        ) -> Result<Self, Self> {
            match atomic.compare_exchange_weak(compare, self, order, Relaxed) {
                Ok(old) => Ok(old),
                Err(_) => Err(self),
            }
        }
        unsafe fn get_atomic(atomic: &mut Self::Atomic) -> Self {
            *atomic.get_mut()
        }
    }
}

impl Atomizable for bool { impl_atomizable!(AtomicBool); }
impl Atomizable for i8 { impl_atomizable!(AtomicI8); }
impl Atomizable for u8 { impl_atomizable!(AtomicU8); }
impl Atomizable for i16 { impl_atomizable!(AtomicI16); }
impl Atomizable for u16 { impl_atomizable!(AtomicU16); }
impl Atomizable for i32 { impl_atomizable!(AtomicI32); }
impl Atomizable for u32 { impl_atomizable!(AtomicU32); }
impl Atomizable for i64 { impl_atomizable!(AtomicI64); }
impl Atomizable for u64 { impl_atomizable!(AtomicU64); }
impl Atomizable for isize { impl_atomizable!(AtomicIsize); }
impl Atomizable for usize { impl_atomizable!(AtomicUsize); }
impl<T> Atomizable for *mut T { impl_atomizable!(AtomicPtr<T>); }

impl<T: Raw> Atomizable for T {
    type Atomic = <T::Target as Atomizable>::Atomic;
    type Raw = <T::Target as Atomizable>::Raw;
    fn as_raw(&self) -> Self::Raw { Atomizable::as_raw(&Raw::as_raw(self)) }
    fn into_atomic(self) -> Self::Atomic { self.into_raw().into_atomic() }
    unsafe fn load_atomic(atomic: &Self::Atomic, order: Ordering) -> Self {
        let raw = unsafe { T::Target::load_atomic(atomic, order) };
        unsafe { T::from_raw(raw) }
    }
    fn store_atomic(self, atomic: &Self::Atomic, order: Ordering) {
        self.into_raw().store_atomic(atomic, order)
    }
    fn swap_atomic(self, atomic: &Self::Atomic, order: Ordering) -> Self {
        let raw = self.into_raw().swap_atomic(atomic, order);
        unsafe { T::from_raw(raw) }
    }
    fn compare_swap_strong(
        self, atomic: &Self::Atomic, compare: Self::Raw, order: Ordering
    ) -> Result<Self, Self> {
        match self.into_raw().compare_swap_strong(atomic, compare, order) {
            Ok(raw) => Ok(unsafe { Raw::from_raw(raw) }),
            Err(raw) => Err(unsafe { Raw::from_raw(raw) }),
        }
    }
    fn compare_swap_weak(
        self, atomic: &Self::Atomic, compare: Self::Raw, order: Ordering
    ) -> Result<Self, Self> {
        match self.into_raw().compare_swap_weak(atomic, compare, order) {
            Ok(raw) => Ok(unsafe { Raw::from_raw(raw) }),
            Err(raw) => Err(unsafe { Raw::from_raw(raw) }),
        }
    }
    unsafe fn get_atomic(atomic: &mut Self::Atomic) -> Self {
        let raw = unsafe { T::Target::get_atomic(atomic) };
        unsafe { T::from_raw(raw) }
    }
}

pub struct Atomic<T: Atomizable>(T::Atomic);

impl<T: Atomizable> Atomic<T> {
    pub fn new(t: T) -> Self { Self(t.into_atomic()) }
    pub fn into_inner(mut self) -> T {
        let t = unsafe { T::get_atomic(&mut self.0) };
        mem::forget(self);
        t
    }
    pub fn swap(&self, t: T, order: Ordering) -> T {
        t.swap_atomic(&self.0, order)
    }
    pub fn compare_swap_strong(&self, current: T::Raw, new: T, order: Ordering)
    -> Result<T, T> {
        new.compare_swap_strong(&self.0, current, order)
    }
    pub fn compare_swap_weak(&self, current: T::Raw, new: T, order: Ordering)
    -> Result<T, T> {
        new.compare_swap_weak(&self.0, current, order)
    }
}

impl<T: Atomizable + Copy> Atomic<T> {
    pub fn load(&self, order: Ordering) -> T {
        unsafe { T::load_atomic(&self.0, order) }
    }
    pub fn store(&self, t: T, order: Ordering) {
        t.store_atomic(&self.0, order)
    }
}

impl<T: Atomizable> Drop for Atomic<T> {
    fn drop(&mut self) {
        unsafe { drop(T::get_atomic(&mut self.0)) }
    }
}

impl<T> Atomic<Option<T>> where Option<T>: Atomizable {
    pub fn take(&self, order: Ordering) -> Option<T> { self.swap(None, order) }
}
