#![crate_name = "std"]
#![crate_type = "rlib"]
#![feature(no_std, zero_one)]
#![no_std]

pub use core::ptr;
pub use core::{fmt, slice, num, cmp, ops, marker};

#[allow(non_camel_case_types, dead_code)]
pub mod prelude {
	pub mod v1 {
		use core::ops::{Add, Sub, BitAnd, Not};
		use core::num::One;

	    pub use core::prelude::*;

	    pub use core::ptr::{null, null_mut};
	    pub use core::mem::{size_of, size_of_val, uninitialized, transmute};

		pub type uphys = usize;

		#[allow(missing_copy_implementations)]
		pub struct void {
			dummy: u8
		}

		pub fn offset<T>(ptr: &'static T) -> usize {
		    ptr as *const T as usize
		}

		pub fn align_up<T: Clone + One + Add<Output=T> + Sub<Output=T> + BitAnd<Output=T> + Not<Output=T>>(value: T, mut alignment: T) -> T
		{
			alignment = alignment - One::one();
			(value + alignment.clone()) & !alignment
		}

		pub fn align_down<T: One + Sub<Output=T> + BitAnd<Output=T> + Not<Output=T>>(value: T, alignment: T) -> T
		{
			value & !(alignment - One::one())
		}
	}
}
