use arch;
use arch::console;
use params;
use util::FixVec;
use memory;
use memory::{Page, PhysicalPage};

pub use arch::PAGE_SIZE;

pub const MAX_OVERHEAD: usize = PTL1_SIZE;

pub const PTL1_SIZE: usize = TABLE_ENTRIES * PAGE_SIZE;
pub const PTL2_SIZE: usize = TABLE_ENTRIES * PTL1_SIZE;
pub const PTL3_SIZE: usize = TABLE_ENTRIES * PTL2_SIZE;
pub const PTL4_SIZE: usize = TABLE_ENTRIES * PTL3_SIZE;

pub const PHYSICAL_ALLOCATOR_MEMORY: usize = KERNEL_LOCATION + PTL2_SIZE;
pub const FRAMEBUFFER_START: usize = PHYSICAL_ALLOCATOR_MEMORY + PTL1_SIZE;
pub const CPU_LOCAL_START: usize = FRAMEBUFFER_START + PTL1_SIZE;

const TABLE_ENTRIES: usize = 0x1000 / PTR_BYTES;

pub const PRESENT_BIT: usize = 1usize << 0;
pub const WRITE_BIT: usize = 1usize << 1;
pub const USERMODE_BIT: usize = 1usize << 2;
pub const WRITETHROUGH_BIT: usize = 1usize << 3;
pub const CACHE_DISABLE_BIT: usize = 1usize << 4;
pub const PAT_PTL1_BIT: usize = 1usize << 7;
pub const NX_BIT: usize = 1usize << 63;

pub const NO_CACHE_FLAGS: usize = WRITETHROUGH_BIT | CACHE_DISABLE_BIT | PAT_PTL1_BIT;
pub const R_DATA_FLAGS: usize = NX_BIT | WRITE_BIT | PRESENT_BIT;
pub const RW_DATA_FLAGS: usize = NX_BIT | WRITE_BIT | PRESENT_BIT;

pub const PAGE_FLAGS: usize = 0x80000000000003FF;

pub const UPPER_HALF_BITS: usize = 0xFFFF000000000000;
pub const UPPER_HALF_START: usize = 0xFFFF800000000000;
pub const LOWER_HALF_END: usize = 0x0000800000000000;

pub const KERNEL_LOCATION: usize = 0xFFFFFFFF80000000;

pub const MAPPED_PML1TS: usize = 0xFFFFFF0000000000;
pub const MAPPED_PML2TS: usize = KERNEL_LOCATION - PTL2_SIZE;
pub const MAPPED_PML3TS: usize = KERNEL_LOCATION + PTL1_SIZE * 511;

#[derive(Copy, Clone)]
#[repr(packed)]
struct TableEntry(usize);

type Table = [TableEntry; TABLE_ENTRIES];

unsafe fn load_pml4(pml4t: PhysicalPage) {
    asm! {
        [pml4t.addr() => %rax, use memory]

        mov cr3, rax;
    }
}

fn physical_page_from_table_entry(entry: TableEntry) -> PhysicalPage
{
	let TableEntry(entry) = entry;

	PhysicalPage::new(entry & !(PAGE_FLAGS))
}

pub fn get_physical_page(virtual_address: Page) -> PhysicalPage {
	unsafe {
		physical_page_from_table_entry(*get_page_entry(virtual_address))
	}
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

fn get_page_entry(pointer: Page) -> *mut TableEntry {
	let (ptl4_index, ptl3_index, ptl2_index, ptl1_index) = decode_address(pointer);

	(MAPPED_PML1TS +
		ptl4_index * PTL2_SIZE +
		ptl3_index * PTL1_SIZE +
		ptl2_index * PAGE_SIZE +
		ptl1_index * PTR_BYTES) as *mut TableEntry
}

fn page_table_entry(page: PhysicalPage, flags: usize) -> TableEntry {
	TableEntry(page.addr() | flags)
}

fn map_page_table(pt: &mut Table, start_page_offset: usize, end_page_offset: usize, base: uphys, mut flags: usize) {
	assert_page_aligned!(base);

	flags |= PRESENT_BIT;
	let start_index = align_down(start_page_offset, PAGE_SIZE) / PAGE_SIZE;
	let end_index = align_up(end_page_offset, PAGE_SIZE) / PAGE_SIZE;

	println!("kernel-base {:x}, stop: {:x} flags {:x}", KERNEL_LOCATION + start_index * PAGE_SIZE, KERNEL_LOCATION + end_index * PAGE_SIZE, flags);

	println!("base {:x}, start_index: {} - end_index: {} - start_page_offset: {:x} - end_page_offset: {:x}", base, start_index, end_index, start_page_offset, end_page_offset);

	assert!(start_index < TABLE_ENTRIES);
	assert!(start_index < end_index);
	assert!(end_index < TABLE_ENTRIES);

	for i in start_index..end_index {
		pt[i] = page_table_entry(PhysicalPage::new(base + (i - start_index) * PAGE_SIZE), flags);
	}
}

fn table_entry_from_data(table: &'static Table) -> TableEntry {
	page_table_entry(Page::new(offset(table)).to_physical(), PRESENT_BIT | WRITE_BIT)
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

		println!("segment {:x} - end {:x}", hole.virtual_base, hole.virtual_base + hole.end - hole.base);

		map_page_table(&mut ptl1_kernel, virtual_offset, virtual_offset + hole.end - hole.base, hole.base, flags);
	}

	load_pml4(Page::new(offset(&ptl4_static)).to_physical());

	console::set_buffer(FRAMEBUFFER_START);
}
