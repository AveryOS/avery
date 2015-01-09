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

	fn as_slice(&self) -> &[T] {
		self.raw_data().slice(0, self.len())
	}

	fn iter<'a>(&'a self) -> std::slice::Iter<'a, T> {
		self.as_slice().iter()
	}

	fn iter_mut<'a>(&'a mut self) -> std::slice::IterMut<'a, T> {
		let len = self.len();
		self.mut_raw_data().slice_mut(0, len).iter_mut()
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
					data: unsafe { ::core::mem::uninitialized() }
				}
			}
		}

		impl<T> ::std::ops::Index<usize> for $name<T> {
			type Output = T;

		    fn index<'a>(&'a self, index: &usize) -> &'a T {
		        &self.raw_data()[*index]
		    }
		}

		impl<T> ::std::ops::IndexMut<usize> for $name<T> {
			type Output = T;
			
		    fn index_mut<'a>(&'a mut self, index: &usize) -> &'a mut T {
		        &mut self.mut_raw_data()[*index]
		    }
		}
    )
}

macro_rules! offset_of {
    ($t:ty, $f:ident) => (
		&mut ((*(0us as *mut $t)).$f) as *mut _ as usize
    )
}

macro_rules! assert_page_aligned {
    ($e:expr) => (
    	assert!((($e) & (::arch::PAGE_SIZE - 1)) == 0)
    )
}
