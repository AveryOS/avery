use arch;
use alloc::arc::Arc;
use util::IndexList;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Id(usize);

pub static IDS: IndexList<Arc<Info>> = IndexList::new();

#[derive(Copy, Clone)]
pub struct AddressRange {
	start: usize,
	end: usize,
}

pub struct AddressSpace {
	start: usize,
	end: usize,
	ranges: Vec<AddressRange>,
}

impl AddressSpace {
	fn alloc_at(&mut self, pos: usize, size: usize) -> Option<AddressRange> {
		if pos + size >= self.end {
			return None
		}
		if !self.ranges.iter().all(|range| (pos >= range.start + range.end) || (pos + size <= range.start) ) {
			return None
		}
		let range = AddressRange {
			start: pos,
			end: pos + size,
		};
		match self.ranges.binary_search_by(|e| range.start.cmp(&e.start)) {
			Ok(e) => panic!(),
			Err(i) => self.ranges.insert(i, range),
		}
		Some(range)
	}
}

pub struct Info {
	arch: arch::process::Info,
	space: AddressSpace,
}

pub fn new() -> Arc<Info> {
	let (arch, size) = arch::process::Info::new();
	let space = AddressSpace {
		start: 0,
		end: size,
		ranges: Vec::new(),
	};
	Arc::new(Info {
		arch: arch,
		space: space,
	})
}
