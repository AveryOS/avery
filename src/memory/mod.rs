use arch;
use std::mem;

#[derive(Copy, Clone)]
pub struct Page(usize);

#[derive(Copy, Clone)]
pub struct PhysicalPage(uphys);

pub const PAGE_ZERO: Page = Page(0);
pub const PHYSICAL_PAGE_ZERO: PhysicalPage = PhysicalPage(0);

#[inline]
pub fn offset<T>(obj: *const T, count: usize) -> *const T {
    ((obj as usize) + mem::size_of::<T>() * count) as *const T
}

#[inline]
pub fn offset_mut<T>(obj: *mut T, count: usize) -> *mut T {
    ((obj as usize) + mem::size_of::<T>() * count) as *mut T
}

impl Page {
	pub fn new(ptr: usize) -> Page {
		assert_page_aligned!(ptr);
		Page(ptr)
	}
	pub fn ptr(&self) -> usize {
		let Page(ptr) = *self;
		ptr
	}
	pub fn to_physical(&self) -> PhysicalPage {
		arch::memory::get_physical_page(*self)
	}
}

impl PhysicalPage {
	pub fn new(addr: uphys) -> PhysicalPage {
		assert_page_aligned!(addr);
		PhysicalPage(addr)
	}
	pub fn addr(&self) -> uphys {
		let PhysicalPage(addr) = *self;
		addr
	}
}

pub mod initial;
pub mod physical;

pub unsafe fn initialize() {

}
