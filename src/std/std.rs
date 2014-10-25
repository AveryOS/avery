#![crate_name = "std"]
#![crate_type = "rlib"]
#![feature(globs)]
#![no_std]

extern crate core;

pub use core::ptr;
pub use core::{fmt, slice};

#[allow(non_camel_case_types, dead_code)]
pub mod prelude {
    pub use core::prelude::*;

    pub use core::mem::{size_of, size_of_val, uninitialized, transmute};

	pub type uptr = uint;
	pub type uphys = uint;

	pub struct void {
		dummy: u8
	}

	pub fn offset<T>(ptr: &'static T) -> uptr {
	    ptr as *const T as uptr
	}
}