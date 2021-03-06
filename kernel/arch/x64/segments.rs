use cpu;
use arch;

pub const CODE_SEGMENT: u16 = 0x8;
pub const DATA_SEGMENT: u16 = 0x10;

pub const USER_CODE_SEGMENT: u16 = 0x23;
pub const USER_DATA_SEGMENT: u16 = 0x1b;

#[allow(dead_code)]
#[repr(packed)]
#[derive(Copy, Clone)]
pub struct TaskState {
	reserved_0: u32,
	rsps: [u64; 3],
	reserved_1: u64,
	pub ists: [u64; 7],
	reserved_2: u16,
	reserved_3: u16,
	io_bitmap_offse: u16,
}

pub const TASK_STATE_DEF: TaskState = TaskState {
	reserved_0: 0,
	rsps: [0; 3],
	reserved_1: 0,
	ists: [0; 7],
	reserved_2: 0,
	reserved_3: 0,
	io_bitmap_offse: 0,
};

#[allow(dead_code)]
#[repr(packed)]
#[derive(Copy, Clone)]
struct TaskStateDescriptor {
	desc: Descriptor,
	base_higher: u32,
	reserved_1: u32
}

#[repr(packed)]
#[allow(dead_code)]
#[derive(Copy, Clone)]
struct Descriptor {
	limit_low: u16,
	base_low: u16,
	base_middle: u8,
	access: u8,
	granularity: u8,
	base_high: u8
}

const DESCRIPTOR_DEF: Descriptor = Descriptor {
	limit_low: 0,
	base_low: 0,
	base_middle: 0,
	access: 0,
	granularity: 0,
	base_high: 0
};

#[repr(packed)]
struct GDT	{
	 segments: [Descriptor; 5],
	 tsds: [TaskStateDescriptor; cpu::MAX_CPUS],
}

static mut GDT: GDT = GDT {
	segments: [DESCRIPTOR_DEF; 5],
	tsds: [TaskStateDescriptor {desc: DESCRIPTOR_DEF, base_higher: 0, reserved_1: 0}; cpu::MAX_CPUS]
};

fn set_segment(index: usize, code: bool, usermode: bool) {
	let segment = unsafe { &mut GDT.segments[index] };

	segment.access = 0b10010010 | // preset, user_segment, readable
		((if code { 1 } else { 0 }) << 3) |
		((if usermode { 3 } else { 0 }) << 5);

	segment.granularity = 0b00100000 // long mode
}

fn set_task_segment(tss: &'static TaskState) {
	let segment = unsafe { &mut GDT.tsds[cpu::current().index] };

	let base = u64::coerce(offset(tss));
	let base_low = base.split().0.split();

	segment.desc.base_low = base_low.0;
	segment.desc.base_middle = base_low.1.split().0;
	segment.desc.base_high = base_low.1.split().1;
	segment.base_higher = base.split().1;

	segment.desc.access = 0b11101001; // available, type = 4, preset, privilege_level = 3
	segment.desc.granularity = u8::coerce(size_of::<TaskState>()) - 1;
}

extern {
	fn load_segments(data: usize, code: usize);
}

pub unsafe fn initialize_gdt() {
	set_segment(1, true, false);
	set_segment(2, false, false);
	set_segment(3, false, true);
	set_segment(4, true, true);

	let gdt_ptr = arch::CPUPointer {
		limit: u16::coerce(size_of_val(&GDT)) - 1,
		base: offset(&GDT)
	};

	asm! {
		lgdt {&gdt_ptr => %*m};
	}

	load_segments(DATA_SEGMENT as usize, CODE_SEGMENT as usize);
}

pub unsafe fn setup_tss() {
	let cpu = arch::cpu::current_slow();

	cpu.arch.tss.rsps[0] = u64::coerce(cpu.arch.stack.end);

	set_task_segment(&cpu.arch.tss);

	asm! {
		[offset_of!(GDT, tsds) + size_of::<TaskStateDescriptor>() * cpu::current().index => %ax]

		ltr ax
	}
}
