use arch;
use memory;

pub struct CPU {
	pub index: uptr,
	pub arch: arch::cpu::CPU,
	pub local_pages: *mut memory::Page,
}

pub const LOCAL_PAGE_COUNT: uptr = 1;

pub const MAX_CPUS: uptr = 32;

const CPU_DEF: CPU = CPU {
	index: -1,
	arch: arch::cpu::CPU_DEF,
	local_pages: 0u as *mut memory::Page,
};

pub static mut CPUS: [CPU, ..MAX_CPUS] = [CPU_DEF, ..MAX_CPUS];

pub fn current() -> &'static mut CPU {
	unsafe { &mut CPUS[0] }
}