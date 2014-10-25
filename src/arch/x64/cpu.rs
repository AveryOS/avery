use arch::segments;

pub struct CPU {
	pub tss: segments::TaskState,
	pub stack_end: uptr,
}

pub const CPU_DEF: CPU = CPU {
	tss: segments::TASK_STATE_DEF,
	stack_end: 0,
};

pub fn initialize_basic() {
}
