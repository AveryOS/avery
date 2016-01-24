use arch;
use arch::console;
use params;
use cpu;
use util::FixVec;
use memory;
use memory::{Page, PhysicalPage, Addr, physical};
use spin::Mutex;

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

pub const PRESENT_BIT: Addr = 1 << 0;
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
pub const MAPPED_PML2TS: usize = KERNEL_LOCATION - PTL2_SIZE;
pub const MAPPED_PML3TS: usize = KERNEL_LOCATION + PTL1_SIZE * 511;

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
	static mut ptl3_static: Table;
	static mut ptl2_kernel: Table;

	static mut ptl2_dynamic: Table;
	static mut ptl1_kernel: Table;
	static mut ptl1_physical: Table;
	static mut ptl1_frame: Table;
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

fn map_page_table(pt: &mut Table, start_page_offset: usize, end_page_offset: usize, base: Addr, mut flags: Addr) {
	assert_page_aligned!(base);

	flags |= PRESENT_BIT;
	let start_index = align_down(start_page_offset, PAGE_SIZE) / PAGE_SIZE;
	let end_index = align_up(end_page_offset, PAGE_SIZE) / PAGE_SIZE;

	//println!("kernel-base {:x}, stop: {:x} flags {:x}", KERNEL_LOCATION + start_index * PAGE_SIZE, KERNEL_LOCATION + end_index * PAGE_SIZE, flags);

	//println!("base {:x}, start_index: {} - end_index: {} - start_page_offset: {:x} - end_page_offset: {:x}", base, start_index, end_index, start_page_offset, end_page_offset);

	assert!(start_index < TABLE_ENTRIES);
	assert!(start_index < end_index);
	assert!(end_index < TABLE_ENTRIES);

	for i in start_index..end_index {
		pt[i] = page_table_entry(PhysicalPage::new(base + (i - start_index) as Addr * PHYS_PAGE_SIZE), flags);
	}
}

fn table_entry_from_data(table: &'static Table) -> TableEntry {
	page_table_entry(Page::new(offset(table)).get_physical(), PRESENT_BIT | WRITE_BIT)
}

pub unsafe fn get_pml4_physical() -> PhysicalPage {
	Page::new(offset(&ptl4_static)).get_physical()
}

pub unsafe fn initialize_initial(st: &memory::initial::State)
{
	ptl4_static[511] = table_entry_from_data(&ptl3_static);
	ptl4_static[510] = table_entry_from_data(&ptl4_static); // map ptl4 to itself

	ptl3_static[509] = table_entry_from_data(&ptl4_static); // map ptl3 to ptl4
	ptl3_static[510] = table_entry_from_data(&ptl2_kernel);
	ptl3_static[511] = table_entry_from_data(&ptl2_dynamic);

	ptl2_kernel[0] = table_entry_from_data(&ptl1_kernel);
	ptl2_kernel[511] = table_entry_from_data(&ptl4_static); // map ptl2 to ptl4

	ptl2_dynamic[0] = table_entry_from_data(&ptl1_physical);
	ptl2_dynamic[1] = table_entry_from_data(&ptl1_frame);

	// Map the physical memory allocator

	map_page_table(&mut ptl1_physical, 0, st.overhead, (*st.entry).base, WRITE_BIT | NX_BIT);

	// Map framebuffer to virtual memory

	let (fb, fb_size) = console::get_buffer_info();

	assert!(fb_size < PTL1_SIZE); // Framebuffer too large
	map_page_table(&mut ptl1_frame, 0, fb_size, fb, WRITE_BIT | NX_BIT);

	// Map kernel segments

	for hole in st.info.segments.iter() {
		let mut flags = NX_BIT;

		match hole.kind {
			params::SegmentKind::Module => continue,
			params::SegmentKind::Code => flags &= !NX_BIT,
			params::SegmentKind::Data => flags |= WRITE_BIT,
			params::SegmentKind::ReadOnlyData => (),
		}

		let virtual_offset = hole.virtual_base - KERNEL_LOCATION;

		println!("Segment {:?} {:#x} - {:#x} @ {:#x} - {:#x}", hole.kind, hole.virtual_base, hole.virtual_base + (hole.end - hole.base) as usize, hole.base, hole.end);

		map_page_table(&mut ptl1_kernel, virtual_offset, virtual_offset + (hole.end - hole.base) as usize, hole.base, flags);
	}

	KERNEL_MAPPED = true;

	extern {
		static stack_start: void;
		static stack_end: void;
	}

	// Unmap the stack guard page
	*get_page_entry(&mut *LOCK.lock(), Page::new(offset(&stack_start))) = NULL_ENTRY;

	println!("BSP Stack is {:#x} - {:#x}", offset(&stack_start), offset(&stack_end));

	load_pml4(get_pml4_physical());

	console::set_buffer(FRAMEBUFFER_START);
}
