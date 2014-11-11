use arch;
use arch::console;
use params;
use util::FixVec;
use memory;
use memory::{Page, PhysicalPage};

pub const PAGE_SIZE: uptr = arch::PAGE_SIZE;

pub const MAX_OVERHEAD: uptr = PTL1_SIZE;

pub const PTL1_SIZE: uptr = TABLE_ENTRIES * PAGE_SIZE;
pub const PTL2_SIZE: uptr = TABLE_ENTRIES * PTL1_SIZE;
pub const PTL3_SIZE: uptr = TABLE_ENTRIES * PTL2_SIZE;
pub const PTL4_SIZE: uptr = TABLE_ENTRIES * PTL3_SIZE;

pub const PHYSICAL_ALLOCATOR_MEMORY: uptr = KERNEL_LOCATION + PTL2_SIZE;
pub const FRAMEBUFFER_START: uptr = PHYSICAL_ALLOCATOR_MEMORY + PTL1_SIZE;
pub const CPU_LOCAL_START: uptr = FRAMEBUFFER_START + PTL1_SIZE;

const TABLE_ENTRIES: uptr = 0x1000 / ::core::uint::BYTES;

pub const PRESENT_BIT: uptr = 1u << 0;
pub const WRITE_BIT: uptr = 1u << 1;
pub const USERMODE_BIT: uptr = 1u << 2;
pub const WRITETHROUGH_BIT: uptr = 1u << 3;
pub const CACHE_DISABLE_BIT: uptr = 1u << 4;
pub const PAT_PTL1_BIT: uptr = 1u << 7;
pub const NX_BIT: uptr = 1u << 63;

pub const NO_CACHE_FLAGS: uptr = WRITETHROUGH_BIT | CACHE_DISABLE_BIT | PAT_PTL1_BIT;
pub const R_DATA_FLAGS: uptr = NX_BIT | WRITE_BIT | PRESENT_BIT;
pub const RW_DATA_FLAGS: uptr = NX_BIT | WRITE_BIT | PRESENT_BIT;

pub const PAGE_FLAGS: uptr = 0x80000000000003FF;
	
pub const UPPER_HALF_BITS: uptr = 0xFFFF000000000000;
pub const UPPER_HALF_START: uptr = 0xFFFF800000000000;
pub const LOWER_HALF_END: uptr = 0x0000800000000000;

pub const KERNEL_LOCATION: uptr = 0xFFFFFFFF80000000;

pub const MAPPED_PML1TS: uptr = 0xFFFFFF0000000000;
pub const MAPPED_PML2TS: uptr = KERNEL_LOCATION - PTL2_SIZE;
pub const MAPPED_PML3TS: uptr = KERNEL_LOCATION + PTL1_SIZE * 511;

#[repr(packed)]
struct TableEntry(uptr);

type Table = [TableEntry, ..TABLE_ENTRIES];

unsafe fn load_pml4(pml4t: PhysicalPage) {
    asm! {
        [pml4t.addr() => %rax, use memory]

        mov cr3, rax;
        hm:
        mov r11, rsp;
        mov r13, rsp;
        push rax;
        xchg bx, bx;
        jmp hm;
    }
}

fn physical_page_from_table_entry(entry: TableEntry) -> PhysicalPage
{
	let TableEntry(mut entry) = entry;

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

fn decode_address(pointer: Page) -> (uptr, uptr, uptr, uptr) {
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
		ptl1_index * ::core::uint::BYTES) as *mut TableEntry
}

fn page_table_entry(page: PhysicalPage, flags: uptr) -> TableEntry {
	TableEntry(page.addr() | flags)
}

fn map_page_table(pt: &mut Table, start_page_offset: uptr, end_page_offset: uptr, base: uphys, mut flags: uptr) {
	assert_page_aligned!(base);

	flags |= PRESENT_BIT;
	let start_index = align_down(start_page_offset, PAGE_SIZE) / PAGE_SIZE;
	let end_index = align_up(end_page_offset, PAGE_SIZE) / PAGE_SIZE;

	println!("kernel-base {:x}, stop: {:x} flags {:x}", KERNEL_LOCATION + start_index * PAGE_SIZE, KERNEL_LOCATION + end_index * PAGE_SIZE, flags);

	println!("base {:x}, start_index: {} - end_index: {} - start_page_offset: {:x} - end_page_offset: {:x}", base, start_index, end_index, start_page_offset, end_page_offset)

	assert!(start_index < TABLE_ENTRIES);
	assert!(start_index < end_index);
	assert!(end_index < TABLE_ENTRIES);

	for i in range(start_index, end_index) {
		let TableEntry(t) = page_table_entry(PhysicalPage::new(base + (i - start_index) * PAGE_SIZE), flags);
		println!("addr {:x}, phys: {:x}", KERNEL_LOCATION + i * PAGE_SIZE, t);

		pt[i] = page_table_entry(PhysicalPage::new(base + (i - start_index) * PAGE_SIZE), flags);
	}
}

fn table_entry_from_data(table: &'static Table) -> TableEntry {
	page_table_entry(Page::new(offset(table)).to_physical(), PRESENT_BIT | WRITE_BIT)
}

pub unsafe fn initialize_initial(st: memory::initial::State)
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
			params::SegmentModule => continue,
			params::SegmentCode => flags &= !NX_BIT,
			params::SegmentData => flags |= WRITE_BIT,
			params::SegmentReadOnlyData => (),
		}

		let virtual_offset = hole.virtual_base - KERNEL_LOCATION;

		println!("segment {:x} - end {:x}", hole.virtual_base, hole.virtual_base + hole.end - hole.base);

		map_page_table(&mut ptl1_kernel, virtual_offset, virtual_offset + hole.end - hole.base, hole.base, flags);
	}

	load_pml4(Page::new(offset(&ptl4_static)).to_physical());

	arch::halt();

	console::set_buffer(FRAMEBUFFER_START);
}
