use arch;
use arch::console;
use params;
use cpu;
use util::FixVec;
use memory;
use memory::{Page, PhysicalPage, Addr, physical};
use spin::Mutex;
use std::cell::RefCell;

pub use arch::{PAGE_SIZE, PHYS_PAGE_SIZE};

pub const MAX_OVERHEAD: usize = PTL1_SIZE;

pub const PTL1_SIZE: usize = TABLE_ENTRIES * PAGE_SIZE;
pub const PTL2_SIZE: usize = TABLE_ENTRIES * PTL1_SIZE;
pub const PTL3_SIZE: usize = TABLE_ENTRIES * PTL2_SIZE;
pub const PTL4_SIZE: usize = TABLE_ENTRIES * PTL3_SIZE;

pub const PHYSICAL_ALLOCATOR_MEMORY: usize = KERNEL_LOCATION + PTL2_SIZE;
pub const FRAMEBUFFER_START: usize = PHYSICAL_ALLOCATOR_MEMORY + PTL1_SIZE;
pub const CPU_LOCAL_START: usize = FRAMEBUFFER_START + PTL1_SIZE;

pub const ALLOCATOR_START: usize = CPU_LOCAL_START + cpu::MAX_CPUS * cpu::LOCAL_PAGE_COUNT * PAGE_SIZE;
pub const ALLOCATOR_END: usize = (PHYSICAL_ALLOCATOR_MEMORY - PAGE_SIZE) + PTL2_SIZE;

const TABLE_ENTRIES: usize = 0x1000 / PTR_BYTES;

pub const PRESENT_BIT: Addr = 1;
pub const WRITE_BIT: Addr = 1 << 1;
pub const USERMODE_BIT: Addr = 1 << 2;
pub const WRITETHROUGH_BIT: Addr = 1 << 3;
pub const CACHE_DISABLE_BIT: Addr = 1 << 4;
pub const PAT_PTL1_BIT: Addr = 1 << 7;
pub const NX_BIT: Addr = 1 << 63;

pub const NO_CACHE_FLAGS: Addr = WRITETHROUGH_BIT | CACHE_DISABLE_BIT | PAT_PTL1_BIT;
pub const R_DATA_FLAGS: Addr = NX_BIT | WRITE_BIT | PRESENT_BIT;
pub const RW_DATA_FLAGS: Addr = NX_BIT | WRITE_BIT | PRESENT_BIT;

pub const PAGE_FLAGS: Addr = 0x80000000000003FF;

pub const UPPER_HALF_BITS: usize = 0xFFFF000000000000;
pub const UPPER_HALF_START: usize = 0xFFFF800000000000;
pub const LOWER_HALF_END: usize = 0x0000800000000000;

pub const KERNEL_LOCATION: usize = 0xFFFFFFFF80000000;

pub const MAPPED_PML1TS: usize = 0xFFFFFF0000000000;
pub const MAPPED_PML2TS: usize = 0xFFFFFF7F80000000;
pub const MAPPED_PML3TS: usize = 0xFFFFFF7FBFC00000;
pub const MAPPED_PML4T: usize = 0xFFFFFF7FBFDFE000;

const NULL_ENTRY: TableEntry = TableEntry(0);

pub struct Ops;
pub static LOCK: Mutex<Ops> = Mutex::new(Ops);


#[derive(Copy, Clone)]
#[repr(packed)]
pub struct TableEntry(Addr);

type Table = [TableEntry; TABLE_ENTRIES];

pub fn map_view(address: Page, mut target: PhysicalPage, pages: usize, flags: Addr) {
	let ops = &mut *LOCK.lock();
	for i in 0..pages {
		let page = Page::new(address.ptr() + i * PAGE_SIZE);
		unsafe {
			set_page_entry(ops, page, page_table_entry(target, flags));

			//println!("MAP VIEW @ {:#x} to {:#x}", page.ptr(), target.addr());

			target = PhysicalPage::new(target.addr() + arch::PHYS_PAGE_SIZE);
		}
	}
}

pub fn unmap_view(address: Page, pages: usize) {
	let ops = &mut *LOCK.lock();
	for i in 0..pages {
		let page = Page::new(address.ptr() + i * PAGE_SIZE);

		let page_entry = get_page_entry(ops, page);

		unsafe {
			if entry_present(*page_entry) {
				//println!("UNMAP VIEW @ {:#x} to {:#x}", page.ptr(), physical_page_from_table_entry(*page_entry).addr());

				*page_entry = NULL_ENTRY;

				invalidate_page(page);
			}
		}
	}
}

pub fn map(address: Page, pages: usize, flags: Addr) {
	let ops = &mut *LOCK.lock();
	for i in 0..pages {
		let page = Page::new(address.ptr() + i * PAGE_SIZE);
		unsafe {
			let alloc = physical::allocate_page();
			//println!("MAP PAGE @ {:#x} to {:#x}", page.ptr(), alloc.addr());

			set_page_entry(ops, page, page_table_entry(alloc, flags));
		}
	}
}

pub fn unmap(address: Page, pages: usize) {
	let ops = &mut *LOCK.lock();
	for i in 0..pages {
		let page = Page::new(address.ptr() + i * PAGE_SIZE);

		let page_entry = get_page_entry(ops, page);

		unsafe {
			if entry_present(*page_entry) {
				//println!("UNMAP PAGE @ {:#x} to {:#x}", page.ptr(), physical_page_from_table_entry(*page_entry).addr());

				physical::free_page(physical_page_from_table_entry(*page_entry));
				*page_entry = NULL_ENTRY;

				invalidate_page(page);
			}
		}
	}
}

fn entry_present(entry: TableEntry) -> bool {
	entry.0 & PRESENT_BIT != 0
}

pub fn new_process() -> (usize, PhysicalPage) {
	unsafe {
		let ptl4_index = 0;
		let ptl3 = memory::physical::allocate_page();

		let _ops = &mut *LOCK.lock();

		ptl4_static[ptl4_index] = page_table_entry(ptl3, PRESENT_BIT | WRITE_BIT);

		invalidate_all();

		let ptl3t = &mut *((MAPPED_PML3TS + ptl4_index * PAGE_SIZE) as *mut Table);

		for v in ptl3t.iter_mut() {
			*v = NULL_ENTRY;
		}

		(ptl4_index, ptl3)
	}
}

pub fn ensure_page_entry<'s>(_: &'s mut Ops, pointer: Page) -> &'s mut TableEntry {
	unsafe {
		let (ptl4_index, ptl3_index, ptl2_index, ptl1_index) = decode_address(pointer);

		let ptl3 = &mut *((MAPPED_PML3TS + ptl4_index * PAGE_SIZE) as *mut Table);

		ensure_table_entry(&mut ptl4_static, ptl4_index, ptl3);

		let ptl2 = &mut *((MAPPED_PML2TS + ptl4_index * PTL1_SIZE + ptl3_index * PAGE_SIZE) as *mut Table);

		ensure_table_entry(ptl3, ptl3_index, ptl2);

		let ptl1 = &mut *((MAPPED_PML1TS + ptl4_index * PTL2_SIZE + ptl3_index * PTL1_SIZE + ptl2_index * PAGE_SIZE) as *mut Table);

		ensure_table_entry(ptl2, ptl2_index, ptl1);

		&mut ptl1[ptl1_index]
	}
}

unsafe fn set_page_entry<'s>(ops: &'s mut Ops, address: Page, entry: TableEntry) {
	*ensure_page_entry(ops, address) = entry;

	asm! {
		[use memory]
	}
}

fn ensure_table_entry(table: &mut Table, index: usize, lower: &mut Table) {
	if !entry_present(table[index]) {
		let page = physical::allocate_dirty_page();
		let flags = PRESENT_BIT | WRITE_BIT;

		table[index] = page_table_entry(page, flags);

		*lower = [NULL_ENTRY; TABLE_ENTRIES];
	}
}

unsafe fn invalidate_page(page: Page) {
	asm! {
		[page.ptr() => %rdi, use memory]

		invlpg [rdi]
	}
}

pub fn invalidate_all() {
	unsafe {
		asm! {
			[use rax, use memory]

			mov rax, cr3;
			mov cr3, rax;
		}
	}
}

unsafe fn load_pml4(pml4t: PhysicalPage) {
	asm! {
		[pml4t.addr() => %rax, use memory]

		mov cr3, rax;
	}
}

fn physical_page_from_table_entry(entry: TableEntry) -> PhysicalPage {
	PhysicalPage::new(entry.0 & !(PAGE_FLAGS))
}

pub fn get_physical_page(virtual_address: Page) -> PhysicalPage {
	physical_page_from_table_entry(*get_page_entry(&mut *LOCK.lock(), virtual_address))
}

extern {
	static mut ptl4_static: Table;
}

fn decode_address(pointer: Page) -> (usize, usize, usize, usize) {
	let mut address = pointer.ptr();

	address &= !UPPER_HALF_BITS;

	address >>= 12;

	let ptl1_index = address & (TABLE_ENTRIES - 1);

	address >>= 9;

	let ptl2_index = address & (TABLE_ENTRIES - 1);

	address >>= 9;

	let ptl3_index = address & (TABLE_ENTRIES - 1);

	address >>= 9;

	let ptl4_index = address & (TABLE_ENTRIES - 1);

	(ptl4_index, ptl3_index, ptl2_index, ptl1_index)
}

fn get_page_entry<'s>(_: &'s mut Ops, pointer: Page) -> &'s mut TableEntry {
	let (ptl4_index, ptl3_index, ptl2_index, ptl1_index) = decode_address(pointer);

	unsafe { &mut *((MAPPED_PML1TS +
		ptl4_index * PTL2_SIZE +
		ptl3_index * PTL1_SIZE +
		ptl2_index * PAGE_SIZE +
		ptl1_index * PTR_BYTES) as *mut TableEntry) }
}

static mut KERNEL_MAPPED: bool = false;

fn page_table_entry(page: PhysicalPage, flags: Addr) -> TableEntry {
	extern {
		static low_end: void;
		static kernel_start: void;
		static kernel_end: void;
	}

	if unsafe { KERNEL_MAPPED } &&
			page.addr() >= offset(&low_end) as Addr &&
			page.addr() < (offset(&kernel_end) - offset(&kernel_start) + offset(&low_end)) as Addr {
		panic!("Mapping kernel physical memory! {:#x}", page.addr());
	}

	TableEntry(page.addr() | flags)
}

pub unsafe fn get_pml4_physical() -> PhysicalPage {
	Page::new(offset(&ptl4_static)).get_physical()
}

pub unsafe fn initialize_initial(st: &memory::initial::State)
{
	extern {
		static mut ptables: [Table; 32];
		static low_end: void;
		static kernel_start: void;
		static stack_start: void;
		static stack_end: void;
	}

	let high_offset = offset(&kernel_start) - offset(&low_end);

	let table_index = RefCell::new(0);

	let alloc_table = || -> &'static mut Table {
		let r = &mut ptables[*table_index.borrow()];
		*table_index.borrow_mut() += 1;
		r
	};

	let get_table = |table: &mut Table, index: usize| -> &'static mut Table {
		if !entry_present(table[index]) {
			let new_table = alloc_table() as *mut Table as usize - high_offset;

			table[index] = page_table_entry(PhysicalPage::new(new_table as Addr), PRESENT_BIT | WRITE_BIT);
		}

		&mut *((usize::coerce(physical_page_from_table_entry(table[index]).addr()) + high_offset) as *mut Table)
	};

	let set_entry = |pointer: Page, entry: TableEntry| {
		let (ptl4_index, ptl3_index, ptl2_index, ptl1_index) = decode_address(pointer);

		let ptl3 = get_table(&mut ptl4_static, ptl4_index);
		let ptl2 = get_table(ptl3, ptl3_index);
		let ptl1 = get_table(ptl2, ptl2_index);

		ptl1[ptl1_index] = entry;
	};

	let map = |virtual_start: usize, size: usize, base: Addr, mut flags: Addr| {
		assert_page_aligned!(base);
		assert_page_aligned!(virtual_start);

		flags |= PRESENT_BIT;

		let pages = align_up(size, PAGE_SIZE) / PAGE_SIZE;

		for i in 0..pages {
			set_entry(Page::new(virtual_start + i * PAGE_SIZE), page_table_entry(PhysicalPage::new(base + (i as Addr) * PHYS_PAGE_SIZE), flags));
		}
	};

	// map ptl4 to itself
	ptl4_static[510] = page_table_entry(PhysicalPage::new((offset(&ptl4_static) - high_offset) as Addr), PRESENT_BIT | WRITE_BIT);

	// Map the physical memory allocator

	map(PHYSICAL_ALLOCATOR_MEMORY, st.overhead, (*st.entry).base, WRITE_BIT | NX_BIT);

	// Map framebuffer to virtual memory

	let (fb, fb_size) = console::get_buffer_info();

	assert!(fb_size < PTL1_SIZE); // Framebuffer too large
	map(FRAMEBUFFER_START, fb_size, fb, WRITE_BIT | NX_BIT);

	// Map kernel segments

	for hole in st.info.segments.iter() {
		let mut flags = NX_BIT;

		match hole.kind {
			params::SegmentKind::Module => continue,
			params::SegmentKind::Code => flags &= !NX_BIT,
			params::SegmentKind::Data => flags |= WRITE_BIT,
			params::SegmentKind::ReadOnlyData => (),
		}

		println!("Segment {:?} {:#x} - {:#x} @ {:#x} - {:#x}", hole.kind, hole.virtual_base, hole.virtual_base + (hole.end - hole.base) as usize, hole.base, hole.end);

		map(hole.virtual_base, usize::coerce(hole.end - hole.base), hole.base, flags);
	}

	load_pml4(get_pml4_physical());

	KERNEL_MAPPED = true;

	console::set_buffer(FRAMEBUFFER_START);

	// Unmap the stack guard page
	*get_page_entry(&mut *LOCK.lock(), Page::new(offset(&stack_start))) = NULL_ENTRY;

	println!("BSP Stack is {:#x} - {:#x}", offset(&stack_start), offset(&stack_end));
}
