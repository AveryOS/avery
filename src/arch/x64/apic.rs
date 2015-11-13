use arch;
use memory;
use memory::{PhysicalPage, Addr};
use arch::{interrupts, pit};
use cpu;

pub enum Message {
	Fixed,
	LowestPriority,
	SMI,
	RemoteRead,
	NMI,
	Init,
	Startup,
	External
}

const TIMER_VECTOR: u32 = 33;

#[export_name = "apic_registers"]
pub static mut REGISTERS: usize = 0;

const BASE_REGISTER: u32 = 0x1B;

const REG_ID: usize = 0x20;
const REG_VERSION: usize = 0x30;
const REG_EOI: usize = 0xB0;
const REG_SIV: usize = 0xF0;
const REG_TASK_PRIORITY: usize = 0x80;
const REG_ICRL: usize = 0x300;
const REG_ICRH: usize = 0x310;
const REG_LVT_TIMER: usize = 0x320;
const REG_LVT_THERMAL: usize = 0x330;
const REG_LVT_PERF: usize = 0x340;
const REG_LVT_LINT0: usize = 0x350;
const REG_LVT_LINT1: usize = 0x360;
const REG_LVT_ERROR: usize = 0x370;
const REG_TIMER_INIT: usize = 0x380;
const REG_TIMER_CURRENT: usize = 0x390;
const REG_TIMER_DIV: usize = 0x3E0;
const REG_LDR: usize = 0xD0;
const REG_DFR: usize = 0xE0;

const MSR_ENABLE_BIT: u64 = 1 << 11;

const SW_ENABLE: u32 = 1 << 8;
const LVT_MASK: u32 = 1 << 16;
const PERIODIC_TIMER: u32 = 1 << 17;

const MT_NMI: u32 = 4 << 8;

unsafe fn get_reg(offset: usize) -> u32 {
	volatile_load((REGISTERS + offset) as *mut u32)
}

unsafe fn reg(offset: usize, val: u32) {
	volatile_store((REGISTERS + offset) as *mut u32, val);
}

pub unsafe fn eoi() {
	reg(REG_EOI, 0);
}

pub unsafe fn ipi(target: usize, kind: Message, vector: usize) {
	reg(REG_ICRH, (target as u32) << 24);
	reg(REG_ICRL, ((vector as u32) & 0xFF) | (((kind as u32) & 7) << 8));
}

pub unsafe fn local_id() -> u8 {
	get_reg(REG_ID) as u8
}

pub unsafe fn initialize(register_base: Option<Addr>) {
	let registers_physical = register_base.unwrap_or_else(|| (((arch::read_msr(BASE_REGISTER) >> 12) & 0xFFFFFFFFFF) << 12));

	println!("APIC regs {:#x}", registers_physical);

	REGISTERS = memory::map_physical(PhysicalPage::new(registers_physical), 1, arch::memory::RW_DATA_FLAGS | arch::memory::NO_CACHE_FLAGS).1.ptr();

	initialize_ap();
}

pub unsafe fn initialize_ap() {
	reg(REG_DFR, -1);
	reg(REG_LDR, get_reg(REG_LDR) & 0x00FFFFFF);
	reg(REG_LVT_TIMER, LVT_MASK);
	reg(REG_LVT_THERMAL, LVT_MASK);
	reg(REG_LVT_PERF, LVT_MASK);
	reg(REG_LVT_LINT0, LVT_MASK);
	reg(REG_LVT_LINT1, LVT_MASK);
	reg(REG_TASK_PRIORITY, 0);

	arch::write_msr(BASE_REGISTER, arch::read_msr(BASE_REGISTER) | MSR_ENABLE_BIT);

	reg(REG_SIV, 0xFF | SW_ENABLE);
}


#[export_name = "apic_calibrate_ticks"]
pub static mut CALIBRATE_TICKS: u64 = 0;

extern fn calibrate_oneshot(_: &interrupts::Info, _: u8, _: usize) {
	panic!("APIC timer calibration failed. Timer too fast.");
}

extern {
	fn apic_calibrate_pit_handler();
}

static mut pit_gate: interrupts::Gate = interrupts::GATE_DEF;

pub unsafe fn calibrate() {
	let gate = interrupts::ref_gate(pit::VECTOR);
	pit_gate = *gate;
	interrupts::set_gate(pit::VECTOR, apic_calibrate_pit_handler);

	interrupts::register_handler(TIMER_VECTOR as u8, calibrate_oneshot);

	calibrate_ap();
}

pub unsafe fn calibrate_done() {
	*interrupts::ref_gate(pit::VECTOR) = pit_gate;

	for cpu in cpu::cpus() {
		println!("[CPU {}] APIC tick rate: {}", cpu.index, cpu.arch.apic_tick_rate);
	}
}

pub unsafe fn calibrate_ap() {
	reg(REG_TIMER_DIV, 2);
	reg(REG_TIMER_INIT, -1);
	reg(REG_LVT_TIMER, LVT_MASK);

	interrupts::enable();

	let mut current_tick;

	loop
	{
		current_tick = volatile_load(&CALIBRATE_TICKS);

		if current_tick <= current_tick.wrapping_add(2) {
			break
		}
	}

	while volatile_load(&CALIBRATE_TICKS) < current_tick + 1 {}

	reg(REG_LVT_TIMER, TIMER_VECTOR);

	while volatile_load(&CALIBRATE_TICKS) < current_tick + 2 {}

	let ticks = !0 - get_reg(REG_TIMER_CURRENT);

	interrupts::disable();

	reg(REG_LVT_TIMER, LVT_MASK);

	cpu::current().arch.apic_tick_rate = ticks as usize;
}

static mut oneshot_done: bool = false;

extern fn simple_oneshot_wake(_: &interrupts::Info, _: u8, _: usize) {
	unsafe{
		volatile_store(&mut oneshot_done, true);
		oneshot_done = true;
		eoi();
	}
}

pub fn simple_oneshot(ticks: u32) {
	unsafe {
		interrupts::disable();

		interrupts::register_handler(TIMER_VECTOR as u8, simple_oneshot_wake);
		volatile_store(&mut oneshot_done, false);
		reg(REG_TIMER_INIT, ticks);
		reg(REG_LVT_TIMER, TIMER_VECTOR);

		interrupts::enable();

		while !volatile_load(&oneshot_done) {
			arch::pause();
		}
	}
}

/*
extern fn tick(info: &interrupts::Info, _: u8, _: usize) {
{
	if(info.was_kernel())
		Scheduler::schedule();
	else
	{
		Scheduler::lock.enter();

		if(Scheduler::empty())
		else
		{
			Arch::swapgs();

			Thread *old = CPU::current->thread;

			old->registers.general.registers = info.registers;

			Thread *thread = Scheduler::preempt(old);

			Scheduler::lock.leave();

			info.registers = thread->registers.general.registers;

			CPU::current->thread = thread;

			if(thread->owner != old->owner)
				thread->owner->address_space.use();

			Arch::swapgs();
		}
	};

	eoi();
}

fn start_timer() {
	interrupts::register_handler(TIMER_VECTOR, tick);

	reg(reg_timer_init) = CPU::current->apic_tick_rate;
	reg(reg_lvt_timer) = TIMER_VECTOR | periodic_timer;
}*/
