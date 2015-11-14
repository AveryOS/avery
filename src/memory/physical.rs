use arch;
use memory;
use memory::{Addr, PhysicalPage};
use std::slice;
use spin::Mutex;

pub const BITS_PER_UNIT: usize = PTR_BYTES;
pub const BYTE_MAP_SIZE: Addr = BITS_PER_UNIT as Addr * arch::PHYS_PAGE_SIZE;

pub struct Hole {
	base: Addr,
	end: Addr,
	pages: usize,
	bitmap: &'static mut [usize], // NOT THREAD SAFE
}

impl Hole {
	fn clear(&mut self, i: usize) {
		let bit = 1 << (i & (BITS_PER_UNIT - 1));
		self.bitmap[i / BITS_PER_UNIT] &= !bit;
	}

	fn set(&mut self, i: usize) {
		let bit = 1 << (i & (BITS_PER_UNIT - 1));
		self.bitmap[i / BITS_PER_UNIT] |= bit;
	}

	fn get(&self, i: usize) -> bool {
		let bit = 1 << (i & (BITS_PER_UNIT - 1));
		self.bitmap[i / BITS_PER_UNIT] & bit != 0
	}
}

pub static mut HOLES: Mutex<&'static mut [Hole]> = Mutex::new(&mut []); // COMPILER BUG; should be static; ask eddyb

pub fn free_page(page: PhysicalPage) {
	let page = page.addr();

	for hole in unsafe { HOLES.lock().iter_mut() } {
		if page >= hole.base && page < hole.end	{
			let base = hole.base;
			hole.clear(((page - base) / arch::PHYS_PAGE_SIZE) as usize);
			return;
		}
	}

	panic!("Memory doesn't belong to any of the holes");
}

pub fn allocate_dirty_page() -> PhysicalPage {
	use std::intrinsics::cttz;

	for (hole_idx, hole) in unsafe { HOLES.lock().iter_mut().enumerate() } {
		for unit in hole.bitmap.iter_mut() {
			if *unit == !0 {
				continue;
			}
			let bit_idx = unsafe { cttz(!(*unit)) };

			*unit |= 1 << bit_idx;

			return PhysicalPage::new(hole.base + (hole_idx * BITS_PER_UNIT + bit_idx) as Addr * arch::PHYS_PAGE_SIZE);
		}
	}

	panic!("Out of physical memory");
}

pub fn allocate_page() -> PhysicalPage {
	let result = allocate_dirty_page();

	//clear_physical_page(result);

	return result;
}


pub unsafe fn initialize(st: &memory::initial::State) {
	const HOLES_ADDR: *mut Hole = arch::memory::PHYSICAL_ALLOCATOR_MEMORY as *mut Hole;

	let mut _entry = st.list;

	let mut overhead_hole = None;
	let mut pos = memory::offset_mut(HOLES_ADDR, st.holes) as *mut usize;
	let mut hole_index = 0;

	let mut holes = HOLES.lock();

	*holes = slice::from_raw_parts_mut(HOLES_ADDR, st.holes);

	while _entry != null_mut() {
		let entry = &mut *_entry;

		let hole = &mut holes[hole_index];

		if _entry == st.entry {
			overhead_hole = Some(hole_index);
		}

		hole.base = entry.base;
		hole.pages = ((entry.end - entry.base) / arch::PHYS_PAGE_SIZE) as usize;
		hole.end = entry.end;

		let units = div_up(hole.pages, BITS_PER_UNIT);

		hole.bitmap = slice::from_raw_parts_mut(pos, units);

		// Clear pages

		for unit in hole.bitmap.iter_mut() {
			*unit = 0;
		}

		// Set non-existent pages at the end of the word as allocated

		hole.bitmap[units - 1] = !0; // Set all pages at the end as allocated

		for p in ((units - 1) * BITS_PER_UNIT)..hole.pages {
			hole.clear(p);
		}

		pos = memory::offset_mut(pos, units);

		hole_index += 1;
		_entry = entry.next;
	}

	let overhead = pos as usize - arch::memory::PHYSICAL_ALLOCATOR_MEMORY;

	assert!(overhead == st.overhead);

	// Mark overhead as used

	let used = div_up(overhead, arch::PAGE_SIZE);
	let overhead_hole =  &mut holes[overhead_hole.unwrap()];

	for page in 0..used {
		overhead_hole.set(page);
	}
}
