use util::FixVec;
use std;

#[cfg(multiboot)]
pub mod multiboot;
#[cfg(not(multiboot))]
pub mod efi;

pub type Addr = u64;

pub mod dwarf;

pub mod symbols;

pub mod console {
	pub use super::vga::{cls, putc, get_buffer_info, set_buffer};
}

#[allow(dead_code)]
#[repr(packed)]
struct CPUPointer {
	limit: u16,
	base: usize
}

const RFLAGS_BIT_INTERRUPT: usize = 1usize << 9;

const EFER: u32 = 0xC0000080;
const EFER_BIT_SYSCALLS: usize = 1;

const GS_BASE: u32 = 0xC0000101;

pub const PAGE_SIZE: usize = 0x1000;
pub const PHYS_PAGE_SIZE: Addr = PAGE_SIZE as Addr;

#[allow(dead_code)]
#[repr(packed)]
#[derive(Debug)]
struct GeneralRegisters {
	r15: u64,
	r14: u64,
	r13: u64,
	r12: u64,
	r11: u64,
	r10: u64,
	r9: u64,
	r8: u64,
	rdi: u64,
	rcx: u64,
	rbp: u64,
	rbx: u64,
	rax: u64,
	rsi: u64,
	rdx: u64,
	rip: u64,
	cs: u64,
	rflags: u64,
	rsp: u64,
}

pub fn pause() {
	unsafe { asm! { pause } }
}

pub fn halt() {
	unsafe {
		asm! { hlt }
	}
}

pub unsafe fn freeze() -> ! {
	interrupts::disable();
	cpu::current_safe().map(|cpu| cpu.arch.frozen.store(true, std::sync::atomic::Ordering::SeqCst));
	loop {
		halt();
	}
}

unsafe fn run() {
	//APIC::start_timer();

	interrupts::enable();

	loop {
		halt();
	}
}

unsafe fn read_msr(reg: u32) -> u64
{
	let low: u32;
	let high: u32;

	asm! {
		[%eax => low, %edx => high, reg => %ecx]

		rdmsr
	}

	low as u64 | ((high as u64) << 32)
}

unsafe fn write_msr(reg: u32, value: u64)
{
	asm! {
		[value => %eax, value >> 32 => %edx, reg => %ecx]

		wrmsr
	}
}

unsafe fn inb(port: u16) -> u8
{
	let ret: u8;

	asm! {
		in {%al => ret}, {port => %dx}
	}

	ret
}

unsafe fn outb(port: u16, value: u8)
{
	asm! {
		out {port => %dx}, {value => %al}
	}
}

mod serial;
mod vga;
mod acpi;
pub mod apic;
mod pit;
mod io_apic;

pub use self::io_apic::IRQ;

pub mod segments;
pub mod interrupts;
pub mod cpu;
pub mod memory;

pub mod process {
	use arch;
	use memory::PhysicalPage;

	pub struct Info {
		ptl4_i: usize,
		ptl3: PhysicalPage,
		pub base: usize,
	}

	impl Info {
		pub fn new() -> (Info, usize) {
			let (i, p) = arch::memory::new_process();
			(Info {
				ptl4_i: i,
				ptl3: p,
				base: 0
			}, arch::memory::PTL3_SIZE)
		}
	}
}

pub unsafe fn initialize_basic() {
	asm! {
		[use rax]

		// turn on write protect
		mov rax, cr0;
		or rax, {1usize << 16 => %i};
		mov cr0, rax;
	}

	segments::initialize_gdt();
	cpu::initialize_basic();
	interrupts::initialize_idt();
}

pub unsafe fn initialize() {
	use elfloader::{self, elf};
	use std::slice;
	use process;
	use memory::Page;
	use std::mem::transmute;
	use std::ptr::copy_nonoverlapping;

	extern {
		static user_image_start: u8;
		static user_image_end: u8;
	}

	cpu::map_local_page_tables(cpu::bsp());

	let pit_irq = IRQ::new(0, true, false);
	let setup = acpi::initialize(pit_irq);
	apic::initialize(setup.apic_address);
	io_apic::initialize(setup.ios);
	pit::initialize(setup.pit_irq);
	apic::calibrate();
	cpu::boot_cpus(setup.cpus);

	let user = slice::from_raw_parts(&user_image_start, offset(&user_image_end) - offset(&user_image_start));

	let mut process = process::new();

	let bin = elfloader::ElfBinary::new("user_image", user).unwrap();

	for p in bin.program_headers() {
		println!("matching program header {} EXEC:{}", p, p.flags.0 & elf::PF_X.0 != 0);
	}

	for h in bin.section_headers() {
		println!("section_header {} {}", bin.section_name(h), h);
	}

	bin.load(|header, data| {
		println!("loading program header {} EXEC:{}", header, header.flags.0 & elf::PF_X.0 != 0);
		let pos = usize::coerce(header.vaddr);
		let size = usize::coerce(header.memsz);
		let pos_aligned = align_down(pos, PAGE_SIZE);
		let size_aligned = align_up(pos + size, PAGE_SIZE) - pos_aligned;
		process.space.lock().alloc_at(pos_aligned, size);
		memory::map(Page::new(process.arch.base + pos_aligned), size_aligned / PAGE_SIZE, memory::WRITE_BIT | memory::PRESENT_BIT);
		std::ptr::copy_nonoverlapping(data.as_ptr(), (process.arch.base + pos) as *mut u8, data.len());
		Ok(())
	});

	let entry = process.arch.base + usize::coerce(bin.header.entry);

	println!("program entry point: {:x}", entry);

	write_msr(GS_BASE, u64::coerce(process.arch.base));

	asm! {
		[entry => %rax, -1isize => %rbx]
		call rax
	}
}
