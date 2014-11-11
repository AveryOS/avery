use arch;

pub const BITS_PER_UNIT: uptr = ::core::uint::BITS;
pub const BYTE_MAP_SIZE: uptr = BITS_PER_UNIT * arch::PAGE_SIZE;

pub struct Hole {
	base: uphys,
	end: uphys,
	pages: uptr,
	units: uptr,
	bitmap: *mut uptr,
}

pub unsafe fn initialize() {
	
}