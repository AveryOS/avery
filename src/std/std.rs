#![crate_name = "std"]
#![crate_type = "rlib"]
#![feature(no_std, zero_one, core, core_intrinsics, raw, core_simd, num_bits_bytes, macro_reexport)]
#![no_std]

// We want to reexport a few macros from core but libcore has already been
// imported by the compiler (via our #[no_std] attribute) In this case we just
// add a new crate name so we can attach the reexports to it.
#[macro_reexport(assert, assert_eq, debug_assert, debug_assert_eq,
                 unreachable, unimplemented, write, writeln)]
extern crate core as __core;

pub use core::ptr;
pub use core::any;
pub use core::cell;
pub use core::clone;
pub use core::cmp;
pub use core::convert;
pub use core::default;
pub use core::hash;
pub use core::intrinsics;
pub use core::iter;
pub use core::fmt;
pub use core::marker;
pub use core::mem;
pub use core::ops;
pub use core::slice;
pub use core::raw;
#[allow(deprecated)]
pub use core::simd;
pub use core::result;
pub use core::option;

#[allow(non_camel_case_types, dead_code)]
pub mod prelude {
	pub mod v1 {
		use core::ops::{Add, Sub, BitAnd, Not};
		use core::num::One;

	    pub use core::prelude::v1::*;

	    pub use core::ptr::{null, null_mut};
	    pub use core::mem::{size_of, size_of_val, uninitialized, transmute};

		pub type uphys = usize;

		pub const PTR_BYTES: usize = ::core::usize::BYTES;

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
