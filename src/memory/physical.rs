use arch;

pub const BITS_PER_UNIT: usize = ::core::uint::BITS;
pub const BYTE_MAP_SIZE: usize = BITS_PER_UNIT * arch::PAGE_SIZE;

pub struct Hole {
	base: uphys,
	end: uphys,
	pages: usize,
	units: usize,
	bitmap: *mut usize,
}

pub unsafe fn initialize() {
	
}