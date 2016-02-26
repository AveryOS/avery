use arch;
use alloc::arc::Arc;
use util::IndexList;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Id(usize);

pub static IDS: IndexList<Arc<Info>> = IndexList::new();

pub struct AddressRange {
	start: usize,
	end: usize,
}

pub struct AddressSpace {
	start: usize,
	end: usize,
	ranges: Vec<AddressRange>,
}

pub struct Info {
	arch: arch::process::Info
}

pub fn new() -> Arc<Info> {
	let arch = arch::process::Info::new();

	Arc::new(Info {
		arch: arch,
	})
}