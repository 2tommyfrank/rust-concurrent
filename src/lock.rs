mod peterson;
mod filter;
mod bakery;
mod tas;
mod ttas;
mod array;
mod clh;
mod mcs;

use crate::Str;

pub trait Lock: Sized {
    type Ref<'a>: LockRef<'a> where Self: 'a;
    fn borrow(&mut self) -> Result<Self::Ref<'_>, Str>;
}

pub trait BoundedLock: Lock {
    fn with_capacity(max_threads: usize) -> Self;
    fn capacity(&self) -> usize;
    fn refs_left(&self) -> usize;
}

pub trait UnboundedLock: Lock {
    fn new() -> Self;
}

pub trait LockRef<'a>: Send {
    // the guard's drop method should release the lock
    type Guard: Drop;
    fn acquire(&mut self) -> Self::Guard;
}

pub use peterson::{PetersonLock, PetersonRef};
pub use filter::{FilterLock, FilterRef};
pub use bakery::{BakeryLock, BakeryRef};
pub use tas::TasLock;
pub use ttas::{TtasLock, BackoffLock};
pub use array::{ArrayLock, ArrayRef};
pub use clh::ClhLock;
pub use mcs::McsLock;
