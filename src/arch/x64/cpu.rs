use arch::{interrupts, acpi, apic, segments};
use arch;
use util::FixVec;
use memory;
use memory::{Page, PhysicalPage, Addr};
use cpu;
use std;
use std::sync::atomic::{AtomicUsize, AtomicBool};
use std::sync::atomic::Ordering::SeqCst;

const INTERRUPT_STACK_PAGES: usize = 5;

pub struct Stack {
	pub start: usize,
	pub end: usize,
}

pub struct CPU {
	pub tss: segments::TaskState,
	pub stack: Stack,
	pub apic_id: u8,
	pub acpi_id: u8,
	pub apic_tick_rate: usize,
	started: AtomicBool,
	pub frozen: AtomicBool,
	pub has_idt: AtomicBool,
}

impl CPU {
	pub fn new() -> CPU {
		CPU {
			tss: unsafe { std::mem::zeroed() },
			stack: Stack {
				start: 0,
				end: 0,
			},
			apic_id: !0,
			acpi_id: !0,
			apic_tick_rate: 0,
			started: AtomicBool::new(false),
			frozen: AtomicBool::new(false),
			has_idt: AtomicBool::new(false),
		}
	}
}

fn is_local_cpu(cpu: &cpu::CPU) -> bool {
	if cpu.index == 0 {
		// We don't have APIC registers yet, so only the BSP can be running
		if unsafe { apic::REGISTERS } == 0 {
			return true;
		}
		// We don't know the apic id of the BSP yet, so only the BSP can be running
		if !cpu.arch.started.load(SeqCst) {
			return true;
		}
	}
	cpu.arch.apic_id == apic::local_id()
}

fn cpus_frozen() -> bool {
	for cpu in cpu::cpus() {
		if is_local_cpu(cpu) || !cpu.arch.started.load(SeqCst) {
			continue;
		}

		if !cpu.arch.frozen.load(SeqCst) {
			return false;
		}
	}

	true
}

static FREEZING: AtomicBool = AtomicBool::new(false);

pub unsafe fn freeze_other_cores() {
	arch::interrupts::disable();

	// Some other core is freezing before us
	if FREEZING.swap(true, SeqCst) {
		arch::freeze();
	}

	for cpu in cpu::cpus() {
		// Skip local CPUs and CPUs without interrupts ready
		if is_local_cpu(cpu) || !cpu.arch.has_idt.load(SeqCst) {
			continue;
		}

		apic::ipi(cpu.arch.apic_id, apic::Message::NMI, 0);
	}

	while !cpus_frozen() {
		arch::pause();
	}

	FREEZING.store(false, SeqCst);
}

pub unsafe fn setup_gs(cpu: *const cpu::CPU) {
	arch::write_msr(arch::GS_BASE, cpu as u64);
}

pub fn current_slow() -> &'static mut cpu::CPU {
	for cpu in cpu::cpus() {
		if is_local_cpu(cpu) {
			return cpu
		}
	}

	panic!("Unable to find current CPU");
}

pub fn current() -> &'static mut cpu::CPU {
	current_slow()
}

pub unsafe fn bsp() -> &'static mut cpu::CPU {
	&mut cpu::cpus()[0]
}

pub fn map_local_page_tables(cpu: &mut cpu::CPU) {
	for page in 0..cpu::LOCAL_PAGE_COUNT {
		let page = memory::Page::new(cpu.local_pages.ptr() + page * arch::PAGE_SIZE);
		arch::memory::ensure_page_entry(&mut *arch::memory::LOCK.lock(), page);
	}
}

pub unsafe fn initialize_basic() {
	setup_gs(bsp());
}

#[repr(packed)]
struct APBootstrapInfo {
	pml4: u32,
	allow_start: AtomicUsize,
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
	// Move setup code to low memory

	assert!(offset(&ap_bootstrap_end) - offset(&ap_bootstrap_start) <= arch::PAGE_SIZE, "CPU bootstrap code too large");

	let bootstrap_page = &ap_bootstrap_mapped as *const void as usize;

	arch::memory::map_view(Page::new(bootstrap_page), PhysicalPage::new(bootstrap_page as Addr), 1, arch::memory::WRITE_BIT | arch::memory::PRESENT_BIT);

	::rlibc::memcpy(bootstrap_page as *mut u8, &ap_bootstrap_start as *const void as *const u8, arch::PAGE_SIZE);

	// Write bootstrap info

	ap_bootstrap_info = APBootstrapInfo {
		pml4: arch::memory::get_pml4_physical().addr() as u32,
		apic_registers: apic::REGISTERS,
		cpu_count: cpu::cpus().len(),
		cpu_size: size_of::<cpu::CPU>(),
		cpu_apic_offset: offset_of!(cpu::CPU, arch) + offset_of!(CPU, apic_id),
		cpu_stack_offset: offset_of!(cpu::CPU, arch) + offset_of!(CPU, stack) + offset_of!(Stack, end),
		cpus: cpu::cpus().as_mut_ptr(),
		allow_start: AtomicUsize::new(0),
	};

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
		if !cpu.arch.started.load(SeqCst) {
			return false;
		}
	}

	true
}

unsafe fn alloc_stack(pages: usize) -> Stack {
	let (_, stack_page) = memory::alloc_block(pages + 1, memory::Kind::Stack);

	arch::memory::map(Page::new(stack_page.ptr() + arch::PAGE_SIZE), pages, arch::memory::RW_DATA_FLAGS);

	Stack {
		start: stack_page.ptr(),
		end: stack_page.ptr() + (pages + 1) * arch::PAGE_SIZE
	}
}

pub unsafe fn boot_cpus(cpus: cpu::CPUVec<acpi::CPUInfo>) {
	let info = setup_ap_bootstrap();

	bsp().arch.apic_id = apic::local_id();

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

	println!("BSP with apic_id id: {}, acpi id: {}", bsp().arch.apic_id, bsp().arch.acpi_id);

	bsp().arch.started.store(true, SeqCst);

	// Wake up other CPUs

	for cpu in cpu::cpus() {
		// Allocate a stack

		cpu.arch.stack = alloc_stack(10);
		println!("CPU {} Stack {:x} - {:x}", cpu.index, cpu.arch.stack.start, cpu.arch.stack.end);

		let nmi_stack = alloc_stack(INTERRUPT_STACK_PAGES);
		println!("CPU {} NMI Stack {:x} - {:x}", cpu.index, nmi_stack.start, nmi_stack.end);
		cpu.arch.tss.ists[0] = nmi_stack.end as u64;

		let double_fault_stack = alloc_stack(INTERRUPT_STACK_PAGES);
		println!("CPU {} Double Fault Stack {:x} - {:x}", cpu.index, double_fault_stack.start, double_fault_stack.end);
		cpu.arch.tss.ists[1] = double_fault_stack.end as u64;

		let page_fault_stack = alloc_stack(INTERRUPT_STACK_PAGES);
		println!("CPU {} Page Fault Stack {:x} - {:x}", cpu.index, page_fault_stack.start, page_fault_stack.end);
		cpu.arch.tss.ists[2] = page_fault_stack.end as u64;

		if cpu.index == 0 {
			continue
		}

		println!("Starting CPU with apic_id id: {}, acpi id: {}", cpu.arch.apic_id, cpu.arch.acpi_id);

		apic::ipi(cpu.arch.apic_id, apic::Message::Init, 0);
	}

	segments::setup_tss();
	interrupts::setup_fatal_handlers();

	// Sync the CPU structs and bootstrap info for the new CPUs
	std::sync::atomic::fence(SeqCst);

	if cpu::cpus().len() > 1 {
		apic::simple_oneshot(1300000);

		send_startup();
		send_startup();

		println!("Waiting for the CPUs to start...");
	}
	// interrupts are enabled by apic::simple_oneshot

	info.allow_start.store(1, SeqCst);


	// interrupts are disable by apic::calibrate_ap
	//apic::calibrate_ap();

	interrupts::enable();

	while !cpus_started() {
		arch::halt();
	}

	println!("All CPUs have started");

	//Memory::clear_lower();
	apic::calibrate_done();
}

#[no_mangle]
pub unsafe extern fn ap_entry(cpu: &'static mut cpu::CPU) {
	segments::initialize_gdt();
	segments::setup_tss();
	interrupts::load_idt();

	// There is freezing going on during bootup
	if FREEZING.load(SeqCst) {
		arch::freeze();
	}

	setup_gs(cpu);

	map_local_page_tables(cpu);

	apic::initialize_ap();

	apic::calibrate_ap();

	cpu.arch.started.store(true, SeqCst);

	arch::run();
}
