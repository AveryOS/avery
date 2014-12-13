#![crate_name = "std"]
#![crate_type = "rlib"]
#![feature(globs)]
#![no_std]

extern crate core;

pub use core::ptr;
pub use core::{fmt, slice, num, cmp};

#[allow(non_camel_case_types, dead_code)]
pub mod prelude {
	use core::num::Int;

    pub use core::prelude::*;

    pub use core::ptr::{null, null_mut};
    pub use core::mem::{size_of, size_of_val, uninitialized, transmute};


	pub type uptr = uint;
	pub type uphys = uint;

	#[allow(missing_copy_implementations)]
	pub struct void {
		dummy: u8
	}

	pub fn offset<T>(ptr: &'static T) -> uptr {
	    ptr as *const T as uptr
	}


	pub fn align_up<T: Int>(value: T, mut alignment: T) -> T
	{
		alignment = alignment - Int::one();
		(value + alignment) & !alignment
	}

	pub fn align_down<T: Int>(value: T, alignment: T) -> T
	{
		value & !(alignment - Int::one())
	}
}