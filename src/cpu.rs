use arch;
use memory;
use util::FixVec;

#[derive(Copy, Clone)]
pub struct CPU {
	pub index: usize,
	pub arch: arch::cpu::CPU,
	pub local_pages: memory::Page,
}

pub const LOCAL_PAGE_COUNT: usize = 1;

pub const MAX_CPUS: usize = 32;

const CPU_DEF: CPU = CPU {
	index: -1,
	arch: arch::cpu::CPU_DEF,
	local_pages: memory::PAGE_ZERO,
};

pub static mut CPUS: [CPU; MAX_CPUS] = [CPU_DEF; MAX_CPUS];

fix_array_struct!(CPUVec, MAX_CPUS);

pub fn current() -> &'static mut CPU {
	unsafe { &mut CPUS[0] }
}

pub fn setup(cpu: &mut CPU, index: usize) {
	cpu.index = index;
	cpu.local_pages = memory::Page::new(arch::memory::CPU_LOCAL_START + index * arch::PAGE_SIZE * LOCAL_PAGE_COUNT);
}
