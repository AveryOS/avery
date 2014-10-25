use arch::segments;
use arch;
use memory;
use cpu;

pub struct CPU {
	pub tss: segments::TaskState,
	pub stack_end: uptr,
}

pub const CPU_DEF: CPU = CPU {
	tss: segments::TASK_STATE_DEF,
	stack_end: 0,
};

pub unsafe fn setup_gs(cpu: &'static mut cpu::CPU) {
	arch::write_msr(arch::GS_BASE, offset(cpu));
}

pub fn setup(cpu: &mut cpu::CPU, index: uptr) {
	cpu.index = index;
	cpu.local_pages = (arch::memory::CPU_LOCAL_START + index * arch::PAGE_SIZE * cpu::LOCAL_PAGE_COUNT) as *mut memory::Page;
}

pub unsafe fn initialize_basic() {
	setup(&mut cpu::CPUS[0], 0);
	setup_gs(&mut cpu::CPUS[0]);
}
