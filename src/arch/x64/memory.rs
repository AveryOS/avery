use arch;

const TABLE_ENTRIES: uptr = 512;

pub const MAX_OVERHEAD: uptr = PTL1_SIZE;

pub const PTL1_SIZE: uptr = TABLE_ENTRIES * arch::PAGE_SIZE;
pub const PTL2_SIZE: uptr = TABLE_ENTRIES * PTL1_SIZE;

#[allow(dead_code)]
#[repr(packed)]
pub struct Page {
	mem: [u8, ..arch::PAGE_SIZE]
}

pub const KERNEL_LOCATION: uptr = 0xFFFFFFFF80000000;

pub const PHYSICAL_ALLOCATOR_MEMORY: uptr = KERNEL_LOCATION + PTL2_SIZE;
pub const FRAMEBUFFER_START: uptr = PHYSICAL_ALLOCATOR_MEMORY + PTL1_SIZE;
pub const CPU_LOCAL_START: uptr = FRAMEBUFFER_START + PTL1_SIZE;
