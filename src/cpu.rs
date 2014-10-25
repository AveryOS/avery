use arch;

pub struct CPU {
	pub index: uptr,
	pub arch: arch::cpu::CPU,
}

pub const MAX_CPUS: uptr = 32;

static mut CPUS: [CPU, ..MAX_CPUS] = [CPU {index: -1, arch: arch::cpu::CPU_DEF}, ..MAX_CPUS];

pub fn current() -> &'static mut CPU {
	unsafe { &mut CPUS[0] }
}