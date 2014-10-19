#![crate_name = "std"]
#![crate_type = "rlib"]
#![feature(globs)]
#![no_std]

extern crate core;

pub use core::fmt;

pub mod prelude {
    pub use core::prelude::*;
}