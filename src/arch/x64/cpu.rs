use arch::{interrupts, acpi, apic, segments};
use arch;
use util::FixVec;
use memory;
use memory::{Page, PhysicalPage, Addr};
use cpu;
use std;

pub struct CPU {
	pub tss: segments::TaskState,
	pub stack: usize,
	pub stack_end: usize,
	pub apic_id: usize,
	pub acpi_id: usize,
	pub apic_tick_rate: usize,
	started: bool,
}

impl CPU {
	pub fn new() -> CPU {
		CPU {
			tss: unsafe { std::mem::zeroed() },
			stack: 0,
			stack_end: 0,
			apic_id: 0,
			acpi_id: 0,
			apic_tick_rate: 0,
			started: false,
		}
	}
}

pub unsafe fn setup_gs(cpu: *const cpu::CPU) {
	arch::write_msr(arch::GS_BASE, cpu as u64);
}

pub fn current() -> &'static mut cpu::CPU {
	unsafe { bsp() } // NEEDS FIXING
}

pub unsafe fn bsp() -> &'static mut cpu::CPU {
	&mut cpu::cpus()[0]
}

pub fn map_local_page_tables(cpu: &mut cpu::CPU) {
	for page in 0..cpu::LOCAL_PAGE_COUNT {
		let page = memory::Page::new(cpu.local_pages.ptr() + page * arch::PAGE_SIZE);
		arch::memory::ensure_page_entry(page);
	}
}

pub unsafe fn initialize_basic() {
	setup_gs(bsp());
}

#[repr(packed)]
struct APBootstrapInfo {
	pml4: u32,
	allow_start: u32,
	apic_registers: usize,
	cpu_count: usize,
	cpu_size: usize,
	cpu_apic_offset: usize,
	cpu_stack_offset: usize,
	cpus: *mut cpu::CPU,
}

extern {
	static ap_bootstrap: void;
	static ap_bootstrap_start: void;
	static ap_bootstrap_end: void;
	static ap_bootstrap_mapped: void;
	static mut ap_bootstrap_info: APBootstrapInfo;
}

unsafe fn setup_ap_bootstrap() -> &'static mut APBootstrapInfo {
	ap_bootstrap_info = APBootstrapInfo {
		pml4: arch::memory::get_pml4_physical().addr() as u32,
		apic_registers: apic::REGISTERS,
		cpu_count: cpu::cpus().len(),
		cpu_size: size_of::<cpu::CPU>(),
		cpu_apic_offset: offset_of!(cpu::CPU, arch) + offset_of!(CPU, apic_id),
		cpu_stack_offset: offset_of!(cpu::CPU, arch) + offset_of!(CPU, stack_end),
		cpus: cpu::cpus().as_mut_ptr(),
		allow_start: 0,
	};

	// Move setup code to low memory

	assert!(offset(&ap_bootstrap_end) - offset(&ap_bootstrap_start) <= arch::PAGE_SIZE, "CPU bootstrap code too large");

	let bootstrap_page = &ap_bootstrap_mapped as *const void as usize;

	arch::memory::map_view(Page::new(bootstrap_page), PhysicalPage::new(bootstrap_page as Addr), 1, arch::memory::WRITE_BIT | arch::memory::PRESENT_BIT);
	::rlibc::memcpy(bootstrap_page as *mut u8, &ap_bootstrap_start as *const void as *const u8, arch::PAGE_SIZE);

	&mut ap_bootstrap_info
}

unsafe fn send_startup() {
	for cpu in cpu::cpus() {
		if cpu.index == 0 {
			continue;
		}

		apic::ipi(cpu.arch.apic_id, apic::Message::Startup, 0x1);
	}

	apic::simple_oneshot(10000);
}

fn cpus_started() -> bool {
	for cpu in cpu::cpus() {
		if unsafe { !volatile_load(&cpu.arch.started) } {
			return false;
		}
	}

	true
}

unsafe fn processor_setup() {
	//segments::setup_tss();
}

pub unsafe fn boot_cpus(cpus: cpu::CPUVec<acpi::CPUInfo>) {
	let info = setup_ap_bootstrap();

	bsp().arch.apic_id = apic::local_id() as usize;

	let mut found_bsp = false;

	for cpu_info in cpus.iter() {
		if cpu_info.apic_id != bsp().arch.apic_id {
			let cpu = cpu::allocate();
			cpu.arch.acpi_id = cpu_info.acpi_id;
			cpu.arch.apic_id = cpu_info.apic_id;
		} else {
			found_bsp = true;
			bsp().arch.acpi_id = cpu_info.acpi_id;
		}
	}

	assert!(found_bsp, "Didn't find the bootstrap processor in ACPI tables");

	bsp().arch.started = true;

	// Wake up other CPUs

	for cpu in cpu::cpus() {
		// Allocate a stack

		let stack_pages = 5;

		let (stack, stack_page) = memory::alloc_block(stack_pages + 1, memory::Kind::Stack);

		cpu.arch.stack = stack_page.ptr();
		cpu.arch.stack_end = stack_page.ptr() + (stack_pages + 1) * arch::PAGE_SIZE;

		arch::memory::map(Page::new(stack_page.ptr() + arch::PAGE_SIZE), stack_pages, arch::memory::RW_DATA_FLAGS);

		if cpu.index == 0 {
			continue
		}

		cpu.arch.started = false;

		println!("Starting CPU with apic_id id: {}, acpi id: {}", cpu.arch.apic_id, cpu.arch.acpi_id);

		apic::ipi(cpu.arch.apic_id, apic::Message::Init, 0);
	}

	if cpu::cpus().len() > 1 {
		apic::simple_oneshot(1300000);

		send_startup();
		send_startup();

		println!("Waiting for the CPUs to start...");
	}

	interrupts::enable();

	info.allow_start = 1;

	while !cpus_started() {
		arch::pause();
	}

	processor_setup();

	println!("All CPUs have started");

	//Memory::clear_lower();
	apic::calibrate_done();
}

#[no_mangle]
pub unsafe extern fn ap_entry(cpu: &'static mut cpu::CPU) {
	segments::initialize_gdt();

	setup_gs(cpu);

	interrupts::initialize_idt();

	processor_setup();

	map_local_page_tables(cpu);

	apic::initialize_ap();
	apic::calibrate_ap();

	cpu.arch.started = true;

	arch::run();
}
