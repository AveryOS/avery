use arch::segments;
use arch;
use memory;
use cpu;

#[derive(Copy, Clone)]
pub struct CPU {
	pub tss: segments::TaskState,
	pub stack_end: usize,
}

pub const CPU_DEF: CPU = CPU {
	tss: segments::TASK_STATE_DEF,
	stack_end: 0,
};

pub unsafe fn setup_gs(cpu: &'static mut cpu::CPU) {
	arch::write_msr(arch::GS_BASE, offset(cpu));
}

pub unsafe fn bsp() -> &'static mut cpu::CPU {
	&mut cpu::CPUS[0]
}

pub fn map_local_page_tables(cpu: &mut cpu::CPU) {
	for page in 0..cpu::LOCAL_PAGE_COUNT {
		let page = memory::Page::new(cpu.local_pages.ptr() + page * arch::PAGE_SIZE);
		arch::memory::ensure_page_entry(page);
	}
}

pub fn setup(cpu: &mut cpu::CPU, index: usize) {
	cpu.index = index;
	cpu.local_pages = memory::Page::new(arch::memory::CPU_LOCAL_START + index * arch::PAGE_SIZE * cpu::LOCAL_PAGE_COUNT);
}

pub unsafe fn initialize_basic() {
	setup(bsp(), 0);
	setup_gs(bsp());
}
