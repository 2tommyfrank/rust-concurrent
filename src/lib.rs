#![deny(unsafe_op_in_unsafe_fn)]

pub mod lock;
pub mod listset;

mod raw;
mod atomic;
mod guard;
mod backoff;
mod acqrel;
mod hash;

type Str = &'static str;
