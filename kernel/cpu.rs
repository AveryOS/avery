use arch;
use memory;
use util::FixVec;

pub struct CPU {
	pub index: usize,
	pub arch: arch::cpu::CPU,
	pub local_pages: memory::Page,
}

pub const LOCAL_PAGE_COUNT: usize = 1;

pub const MAX_CPUS: usize = 32;

fix_array_struct!(CPUVec, MAX_CPUS);

static mut CPUS: Option<CPUVec<CPU>> = None;

const REG_ID: u32 = 0;
const REG_VERSION: u32 = 1;
const REG_IRQ_START: u32 = 0x10;

const MASK_BIT: u32 = 1 << 16;
const TRIGGER_MODE_BIT: u32 = 1 << 15;
const ACTIVE_LOW_BIT: u32 = 1 << 13;

pub unsafe fn initialize_basic() {
	CPUS = Some(CPUVec::new());
	allocate();
}

pub fn cpus() -> &'static mut [CPU] {
	unsafe {
		match CPUS.as_mut() {
			Some(cpus) => cpus.iter_mut().into_slice(),
			None => &mut []
		}
	}
}

pub unsafe fn allocate() -> &'static mut CPU {
	let cpus = CPUS.as_mut().unwrap();
	let count = cpus.len();
	cpus.push(CPU::new(count));

	&mut cpus[count]
}

pub fn current() -> &'static mut CPU {
	arch::cpu::current()
}

impl CPU {
	pub fn new(index: usize) -> CPU {
		CPU {
			index: index,
			arch: arch::cpu::CPU::new(),
			local_pages: memory::Page::new(arch::memory::CPU_LOCAL_START + index * arch::PAGE_SIZE * LOCAL_PAGE_COUNT),
		}
	}
}
