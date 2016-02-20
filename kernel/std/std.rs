#![crate_name = "std"]
#![crate_type = "rlib"]
#![feature(zero_one, core_intrinsics, raw, num_bits_bytes, lang_items,
	macro_reexport, allow_internal_unstable, core_panic,
	collections, alloc, slice_concat_ext)]
#![no_std]

// We want to reexport a few macros from core but libcore has already been
// imported by the compiler (via our #[no_std] attribute) In this case we just
// add a new crate name so we can attach the reexports to it.
#[macro_reexport(assert, assert_eq, debug_assert, debug_assert_eq,
				 unreachable, unimplemented, write, writeln,
				 try, panic)]
extern crate core as __core;

#[macro_use]
#[macro_reexport(vec, format)]
extern crate collections as core_collections;

extern crate alloc;

pub use core_collections::borrow;
pub use core_collections::fmt;
pub use core_collections::slice;
pub use core_collections::str;
pub use core_collections::string;
pub use core_collections::vec;

pub use alloc::boxed;
pub use alloc::rc;

pub use core::ptr;
pub use core::sync;
pub use core::any;
pub use core::cell;
pub use core::clone;
pub use core::cmp;
pub use core::convert;
pub use core::default;
pub use core::num;
pub use core::hash;
pub use core::intrinsics;
pub use core::iter;
pub use core::marker;
pub use core::mem;
pub use core::ops;
pub use core::raw;
pub use core::result;
pub use core::option;
pub use core::panicking;

#[allow(non_camel_case_types, dead_code)]
pub mod prelude {
	pub mod v1 {
		// Reexported core operators
		pub use marker::{Copy, Send, Sized, Sync};
		pub use ops::{Drop, Fn, FnMut, FnOnce};

		// Reexported functions
		pub use mem::drop;

		// Reexported types and traits
		pub use boxed::Box;
		pub use borrow::ToOwned;
		pub use clone::Clone;
		pub use cmp::{PartialEq, PartialOrd, Eq, Ord};
		pub use convert::{AsRef, AsMut, Into, From};
		pub use default::Default;
		pub use iter::{Iterator, Extend, IntoIterator};
		pub use iter::{DoubleEndedIterator, ExactSizeIterator};
		pub use option::Option::{self, Some, None};
		pub use result::Result::{self, Ok, Err};
		pub use slice::SliceConcatExt;
		pub use string::{String, ToString};
		pub use vec::Vec;

		pub use core::intrinsics::{volatile_store, volatile_load};

		use core::ops::{Add, Sub, BitAnd, Not, Div};
		use core::num::One;

		//pub use core::prelude::v1::*;

		pub use core::ptr::{null, null_mut};
		pub use core::mem::{size_of, size_of_val, uninitialized, transmute};

		pub const PTR_BYTES: usize = 8;

		#[allow(missing_copy_implementations)]
		pub struct void {
			dummy: u8
		}

		pub fn offset<T>(ptr: &'static T) -> usize {
			ptr as *const T as usize
		}

		pub fn div_up<T: Clone + One + Add<Output=T> + Sub<Output=T> + BitAnd<Output=T> + Div<Output=T> + Not<Output=T>>(value: T, alignment: T) -> T
		{
			align_up(value, alignment.clone()) / alignment
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

pub mod io {
	use core;
	use core::cmp;

	pub type Result<T> = core::result::Result<T, Error>;

	#[derive(Debug)]
	pub struct Error;

	pub trait Read {
		fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

		fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
			while !buf.is_empty() {
				match self.read(buf) {
					Ok(0) => break,
					Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
					Err(e) => return Err(e),
				}
			}
			if !buf.is_empty() {
				Err(Error)
			} else {
				Ok(())
			}
		}
	}

	pub struct Cursor<T> {
		inner: T,
		pos: u64,
	}

	impl<T> Cursor<T> {
		pub fn new(inner: T) -> Cursor<T> {
			Cursor { pos: 0, inner: inner }
		}

		pub fn into_inner(self) -> T { self.inner }
		pub fn get_ref(&self) -> &T { &self.inner }
		pub fn get_mut(&mut self) -> &mut T { &mut self.inner }
		pub fn position(&self) -> u64 { self.pos }
		pub fn set_position(&mut self, pos: u64) { self.pos = pos; }

	}

	impl<T: AsRef<[u8]>> Cursor<T> {
		fn fill_buf(&mut self) -> Result<&[u8]> {
			let amt = cmp::min(self.pos, self.inner.as_ref().len() as u64);
			Ok(&self.inner.as_ref()[(amt as usize)..])
		}
	}

	impl<T> Read for Cursor<T> where T: AsRef<[u8]> {
		fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
			let n = try!(Read::read(&mut try!(self.fill_buf()), buf));
			self.pos += n as u64;
			Ok(n)
		}
	}

	impl<'a> Read for &'a [u8] {
		#[inline]
		fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
			let amt = cmp::min(buf.len(), self.len());
			let (a, b) = self.split_at(amt);
			buf.clone_from_slice(a);
			*self = b;
			Ok(amt)
		}

		#[inline]
		fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
			if buf.len() > self.len() {
				return Err(Error);
			}
			let (a, b) = self.split_at(buf.len());
			buf.clone_from_slice(a);
			*self = b;
			Ok(())
		}
	}

}
