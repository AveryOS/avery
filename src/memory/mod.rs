use arch;
use std::mem;

pub use arch::Addr;

#[derive(Copy, Clone)]
pub struct Page(usize);

#[derive(Copy, Clone)]
pub struct PhysicalPage(Addr);

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
	pub fn get_physical(&self) -> PhysicalPage {
		arch::memory::get_physical_page(*self)
	}
}

impl PhysicalPage {
	pub fn new(addr: Addr) -> PhysicalPage {
		assert_page_aligned!(addr);
		PhysicalPage(addr)
	}
	pub fn addr(&self) -> Addr {
        self.0
	}
}

pub mod allocator;
pub mod initial;
pub mod physical;

static mut alloc: Option<allocator::Allocator> = None;

pub unsafe fn initialize() {
    alloc = Some(allocator::Allocator::new(Page::new(arch::memory::ALLOCATOR_START), Page::new(arch::memory::ALLOCATOR_END)));
}

pub fn alloc_pages(pages: usize, kind: allocator::Kind) -> Page {
    unsafe { Page::new((*alloc.as_mut().unwrap().allocate(kind, pages)).base * arch::PAGE_SIZE) }
}
