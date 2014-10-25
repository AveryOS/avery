use std;

pub trait FixVec<T> {
	fn mut_raw_data(&mut self) -> &mut [T];
	fn mut_len(&mut self) -> &mut uint;
	fn raw_data(&self) -> &[T];
	fn len(&self) -> uint;

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

	fn iter<'a>(&'a self) -> std::slice::Items<'a, T> {
		self.raw_data().iter()
	}

	fn new() -> Self;
}

macro_rules! fix_array_struct(
    ($name:ident, $c:expr) => (
		#[repr(C)]
		pub struct $name<T> {
			len: uint,
			data: [T, ..$c]
		}

		impl<T> ::util::FixVec<T> for $name<T> {
			fn raw_data(&self) -> &[T] {
				&self.data
			}
			fn mut_raw_data(&mut self) -> &mut [T] {
				&mut self.data
			}
			fn len(&self) -> uint {
				self.len
			}
			fn mut_len(&mut self) -> &mut uint{
				&mut self.len
			}

			fn new() -> $name<T> {
				$name {
					len: 0,
					data: unsafe { ::core::mem::uninitialized() }
				}
			}
		}

		impl<T> Index<uint,T> for $name<T> {
		    fn index<'a>(&'a self, index: &uint) -> &'a T {
		        &self.raw_data()[*index]
		    }
		}

		impl<T> IndexMut<uint,T> for $name<T> {
		    fn index_mut<'a>(&'a mut self, index: &uint) -> &'a mut T {
		        &mut self.mut_raw_data()[*index]
		    }
		}
    )
)

macro_rules! offset_of(
    ($t:ty, $f:ident) => (
		&mut ((*(0u as *mut $t)).$f) as *mut _ as uptr
    )
)
