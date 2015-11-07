use arch;
use memory;
use std::slice;

pub const BITS_PER_UNIT: usize = PTR_BYTES;
pub const BYTE_MAP_SIZE: usize = BITS_PER_UNIT * arch::PAGE_SIZE;

pub struct Hole {
	base: uphys,
	end: uphys,
	pages: usize,
	bitmap: &'static mut [usize],
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

const HOLES: *mut Hole = arch::memory::PHYSICAL_ALLOCATOR_MEMORY as *mut Hole;

pub unsafe fn initialize(st: &memory::initial::State) {
	let mut _entry = st.list;

	let mut overhead_hole = None;
	let mut hole_count = 0;
	let mut pos = memory::offset_mut(HOLES, hole_count) as *mut usize;

	while _entry != null_mut() {
		let entry = &mut *_entry;

		let hole = &mut *memory::offset_mut(HOLES, hole_count);

		if _entry == st.entry {
			overhead_hole = Some(hole_count);
		}

		hole.base = entry.base;
		hole.pages = (entry.end - entry.base) / arch::PAGE_SIZE;
		hole.end = entry.end;

		let units = align_up(hole.pages, BITS_PER_UNIT) / BITS_PER_UNIT;

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

		hole_count += 1;
		_entry = entry.next;
	}

	// Mark overhead as used

	let used = (align_up(pos as usize, arch::PAGE_SIZE) - arch::memory::PHYSICAL_ALLOCATOR_MEMORY) / arch::PAGE_SIZE;
	let overhead_hole = &mut *memory::offset_mut(HOLES, overhead_hole.unwrap());

	for page in 0..used {
		overhead_hole.set(page);
	}
}
