use arch;
use memory;
use memory::{PhysicalPage, Addr};

enum MessageKind {
	Fixed,
	LowestPriority,
	SMI,
	RemoteRead,
	NMI,
	Init,
	Startup,
	External
}

const TIMER_VECTOR: usize = 33;

#[export_name = "apic_registers"]
pub static mut REGISTERS: usize = 0;

#[export_name = "apic_calibrate_ticks"]
pub static mut CALIBRATE_TICKS: u64 = 0;

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

unsafe fn ipi(target: usize, kind: MessageKind, vector: usize) {
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

unsafe fn initialize_ap() {
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

/*
fn calibrate_oneshot(Interrupts::Info &, uint8_t, size_t) {
	panic("APIC timer calibration failed. Timer too fast.");
}

extern {
	fn apic_calibrate_pit_handler();
}

Interrupts::InterruptGate pit_gate;

fn calibrate() {
	Interrupts::get_gate(PIT::vector, pit_gate);
	Interrupts::set_gate(PIT::vector, &apic_calibrate_pit_handler);

	Interrupts::register_handler(timer_vector, calibrate_oneshot);

	calibrate_ap();
}

fn calibrate_done() {
	Interrupts::set_gate(PIT::vector, pit_gate);

	for i in 0..CPU::count {
		console.s("[CPU ").u(i).s("] APIC tick rate: ").u(CPU::cpus[i].apic_tick_rate).endl();
	}
}

fn calibrate_ap()
{
	reg(reg_timer_div) = 2;
	reg(reg_timer_init) = -1;
	reg(reg_lvt_timer) = lvt_mask;

	Interrupts::enable();

	uint64_t current_tick;

	do
	{
		current_tick = calibrate_ticks;
	} while(current_tick > current_tick + 2);

	while(calibrate_ticks < current_tick + 1);

	reg(reg_lvt_timer) = timer_vector;

	while(calibrate_ticks < current_tick + 2);

	uint32_t ticks = (uint32_t)-1 - reg(reg_timer_current);

	Interrupts::disable();

	reg(reg_lvt_timer) = lvt_mask;

	CPU::current->apic_tick_rate = ticks;
}

volatile bool oneshot_done;

fn simple_oneshot_wake(Interrupts::Info &, uint8_t, size_t)
{
	oneshot_done = true;
	eoi();
}

fn simple_oneshot(size_t ticks)
{
	Interrupts::disable();

	Interrupts::register_handler(timer_vector, simple_oneshot_wake);
	oneshot_done = false;
	reg(reg_timer_init) = ticks;
	reg(reg_lvt_timer) = timer_vector;

	Interrupts::enable();

	while(!oneshot_done)
		Arch::pause();
}

fn tick(Interrupts::Info &info, uint8_t, size_t)
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

fn start_timer()
{
	Interrupts::register_handler(timer_vector, tick);

	reg(reg_timer_init) = CPU::current->apic_tick_rate;
	reg(reg_lvt_timer) = timer_vector | periodic_timer;
}*/
