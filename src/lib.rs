#![deny(unsafe_op_in_unsafe_fn)]

pub mod listset;
pub mod lock;

mod atomic;
mod backoff;
mod guard;
mod hash;
mod notify;

type Str = &'static str;
