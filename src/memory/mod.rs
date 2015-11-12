use arch;
use std::mem;
use std::slice;

pub use arch::Addr;
pub use self::allocator::Block;

pub struct PhysicalView {
    block: Option<*mut allocator::Block>,
}

impl PhysicalView {
    pub unsafe fn map<'s>(&'s mut self, base: Addr, size: usize, flags: Addr) -> &'s [u8] {
        let start = align_down(base, arch::PHYS_PAGE_SIZE);
        let end = align_up(base + size as Addr, arch::PHYS_PAGE_SIZE);
        let pages = ((end - start) / arch::PHYS_PAGE_SIZE) as usize;

        let (block, page) = map_physical(PhysicalPage::new(start), pages, flags);

        self.block = Some(block);

        let start = page.ptr() + (base & (arch::PHYS_PAGE_SIZE - 1)) as usize;

        slice::from_raw_parts(start as *const u8, size)
    }

    pub unsafe fn map_object<'s, T>(&'s mut self, base: Addr, flags: Addr) -> &'s T {
        let view = self.map(base, mem::size_of::<T>(), flags);
        &*(view.as_ptr() as *const T)
    }

    pub fn new() -> PhysicalView {
        PhysicalView {
            block: None,
        }
    }
}

impl Drop for PhysicalView {
    fn drop(&mut self) {
        if let Some(block) = self.block {
            unsafe {
                let base = Page::new((*block).base * arch::PAGE_SIZE);
                arch::memory::unmap_view(base, (*block).pages);

                free_block(block);
            }

        }
    }
}

pub fn map_physical(base: PhysicalPage, pages: usize, flags: Addr) -> (*mut Block, Page) {
    let (block, page) = alloc_block(pages, allocator::Kind::PhysicalView);

    arch::memory::map_view(page, base, pages, flags);

	(block, page)
}

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
    static mut alloc_first_block: Option<allocator::Block> = None;
    alloc = Some(allocator::Allocator::new(Page::new(arch::memory::ALLOCATOR_START), Page::new(arch::memory::ALLOCATOR_END), &mut alloc_first_block));
}

pub fn alloc_block(pages: usize, kind: allocator::Kind) -> (*mut allocator::Block, Page) {
    unsafe {
        let block = alloc.as_mut().unwrap().allocate(kind, pages);
        (block, Page::new((*block).base * arch::PAGE_SIZE))
    }
}

pub unsafe fn free_block(block: *mut allocator::Block) {
    alloc.as_mut().unwrap().free(block)
}

pub fn virtual_dump() {
    unsafe { alloc.as_mut().unwrap().dump() }
}
