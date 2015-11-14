use std;

pub trait FixVec<T> {
	fn mut_raw_data(&mut self) -> &mut [T];
	fn mut_len(&mut self) -> &mut usize;
	fn raw_data(&self) -> &[T];
	fn len(&self) -> usize;

	fn push(&mut self, val: T) {
		let idx = self.len();
		unsafe {
			std::ptr::write(&mut self.mut_raw_data()[idx] as *mut T, val);
		}
		*self.mut_len() = idx + 1;
	}

	fn iter<'a>(&'a self) -> std::slice::Iter<'a, T> {
		self.raw_data()[0..self.len()].iter()
	}

	fn iter_mut<'a>(&'a mut self) -> std::slice::IterMut<'a, T> {
		let len = self.len();
		self.mut_raw_data()[0..len].iter_mut()
	}

	fn new() -> Self;
}

macro_rules! fix_array_struct {
	($name:ident, $c:expr) => (
		#[repr(C)]
		pub struct $name<T> {
			len: usize,
			data: [T; $c]
		}

		impl<T> ::util::FixVec<T> for $name<T> {
			fn raw_data(&self) -> &[T] {
				&self.data
			}
			fn mut_raw_data(&mut self) -> &mut [T] {
				&mut self.data
			}
			fn len(&self) -> usize {
				self.len
			}
			fn mut_len(&mut self) -> &mut usize{
				&mut self.len
			}

			fn new() -> $name<T> {
				$name {
					len: 0,
					data: unsafe { ::std::mem::uninitialized() }
				}
			}
		}

		impl<T> ::std::ops::Index<usize> for $name<T> {
			type Output = T;

			fn index<'a>(&'a self, index: usize) -> &'a T {
				&self.raw_data()[index]
			}
		}

		impl<T> ::std::ops::IndexMut<usize> for $name<T> {
			fn index_mut<'a>(&'a mut self, index: usize) -> &'a mut T {
				&mut self.mut_raw_data()[index]
			}
		}
	)
}

pub struct LinkedList<T> {
	pub first: Option<*mut T>,
	pub last: Option<*mut T>,
	prev_offset: usize,
	next_offset: usize,
}

unsafe fn get<T>(node: *mut T, offset: usize) -> *mut Option<*mut T> {
	(node as usize + offset) as *mut Option<*mut T>
}

unsafe fn insert_position<T>(node: *mut T, offset_a: usize, offset_b: usize, abs: &mut Option<*mut T>, target: *mut T) {
	*get(node, offset_a) = Some(target);

	let target_attr = *get(target, offset_b);

	match target_attr {
		Some(target_attr) => {
			*get(target_attr, offset_a) = Some(node);
		}
		None => {
			*abs = Some(node);
		}
	};

	*get(node, offset_b) = target_attr;
	*get(target, offset_b) = Some(node);
}

impl<T> LinkedList<T> {
	pub fn new(prev_offset: usize, next_offset: usize) -> LinkedList<T> {
		LinkedList {
			first: None,
			last: None,
			prev_offset: prev_offset,
			next_offset: next_offset,
		}
	}

	unsafe fn next(&self, node: *mut T) -> *mut Option<*mut T> {
		get(node, self.next_offset)
	}

	unsafe fn prev(&self, node: *mut T) -> *mut Option<*mut T> {
		get(node, self.prev_offset)
	}

	pub unsafe fn append(&mut self, node: *mut T) {
		*self.next(node) = None;

		match self.last {
			Some(prev_last) => {
				*self.prev(node) = Some(prev_last);
				*self.next(prev_last) = Some(node);
				self.last = Some(node);
			}
			None => {
				*self.prev(node) = None;

				self.first = Some(node);
				self.last = Some(node);

			}
		}
	}

	pub unsafe fn remove(&mut self, node: *mut T) {
		match *self.prev(node) {
			Some(val) => {
				*self.next(val) = *self.next(node);
			}
			None => {
				self.first = *self.next(node);
			}
		}

		match *self.next(node) {
			Some(val) => {
				*self.prev(val) = *self.prev(node);
			}
			None => {
				self.last = *self.prev(node);
			}
		}
	}

	pub unsafe fn insert_before(&mut self, node: *mut T, before: *mut T) {
		insert_position(node, self.next_offset, self.prev_offset, &mut self.first, before);
	}

	pub unsafe fn insert_after(&mut self, node: *mut T, after: *mut T) {
		insert_position(node, self.prev_offset, self.next_offset, &mut self.last, after);
	}
}

macro_rules! offset_of {
	($t:ty, $f:ident) => ({
		fn dummy() -> usize { // Get rid of unused unsafe warnings
			unsafe { &mut ((*(0usize as *mut $t)).$f) as *mut _ as usize }
		}
		dummy()
	})
}

macro_rules! assert_page_aligned {
	($e:expr) => (
		assert!((($e) as ::arch::Addr & (::arch::PHYS_PAGE_SIZE - 1)) == 0)
	)
}
