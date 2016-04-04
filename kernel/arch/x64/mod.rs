use util::FixVec;
use std;
use std::fmt;
use elfloader::{self, Image, elf};

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
struct GeneralRegisters {
	r15: usize,
	r14: usize,
	r13: usize,
	r12: usize,
	r11: usize,
	r10: usize,
	r9: usize,
	r8: usize,
	rdi: usize,
	rcx: usize,
	rbp: usize,
	rbx: usize,
	rax: usize,
	rsi: usize,
	rdx: usize,
	rip: usize,
	cs: usize,
	rflags: usize,
	rsp: usize,
}

impl fmt::Display for GeneralRegisters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rip: {:>#18x} rsp: {:>#18x} rax: {:>#18x}\n", self.rip, self.rsp, self.rax)?;
        write!(f, "rbx: {:>#18x} rcx: {:>#18x} rdx: {:>#18x}\n", self.rbx, self.rcx, self.rdx)?;
        write!(f, "rdi: {:>#18x} rsi: {:>#18x} rbp: {:>#18x}\n", self.rdi, self.rsi, self.rbp)?;
        write!(f, "r8:  {:>#18x} r9:  {:>#18x} r10: {:>#18x}\n", self.r8, self.r9, self.r10)?;
        write!(f, "r11: {:>#18x} r12: {:>#18x} r13: {:>#18x}\n", self.r11, self.r12, self.r13)?;
        write!(f, "r14: {:>#18x} r15: {:>#18x}\n", self.r14, self.r15)?;
		Ok(())
    }
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

pub fn get_user_elf() -> Image<'static> {
	use std::slice;

	extern {
		static user_image_start: u8;
		static user_image_end: u8;
	}

	let user = unsafe {
		slice::from_raw_parts(&user_image_start, offset(&user_image_end) - offset(&user_image_start))
	};

	elfloader::Image::new(user).unwrap()
}

pub unsafe fn initialize(st: &::memory::initial::State) {
	use process;
	use memory::Page;
	use std::mem::transmute;
	use std::ptr::copy_nonoverlapping;

	symbols::setup(&st.info.symbols);

	cpu::map_local_page_tables(cpu::bsp());

	let pit_irq = IRQ::new(0, true, false);
	let setup = acpi::initialize(pit_irq);
	apic::initialize(setup.apic_address);
	io_apic::initialize(setup.ios);
	pit::initialize(setup.pit_irq);
	apic::calibrate();
	cpu::boot_cpus(setup.cpus);

	let mut process = process::new();

	let bin = get_user_elf();

	bin.load(|header, data| {
		println!("loading program header {} EXEC:{}", header, header.flags.0 & elf::PF_X.0 != 0);
		let pos = usize::coerce(header.vaddr);
		let size = usize::coerce(header.memsz);
		let pos_aligned = align_down(pos, PAGE_SIZE);
		let size_aligned = align_up(pos + size, PAGE_SIZE) - pos_aligned;
		process.space.lock().alloc_at(pos_aligned, size);
		memory::map(Page::new(process.arch.base + pos_aligned), size_aligned / PAGE_SIZE, memory::WRITE_BIT | memory::PRESENT_BIT);
		std::ptr::copy_nonoverlapping(data.as_ptr(), (process.arch.base + pos) as *mut u8, data.len());
		std::ptr::write_bytes((process.arch.base + pos + data.len()) as *mut u8, 0, size - data.len());
		Ok(())
	});

	let entry = process.arch.base + usize::coerce(bin.header.unwrap().entry);

	println!("program entry point: {:x} stack {:x}", entry, cpu::current().arch.stack.end);

	write_msr(GS_BASE, u64::coerce(process.arch.base));

	asm! {
		[use rax]

		mov rax, cr0;
		// clear emulation, task switch bits
		and rax, {!((1u32 << 2) | (1u32 << 3)) => %i};
		// set monitor coprocessor and native error bits
		or rax, {(1u32 << 1) | (1u32 << 5) => %i};
		mov cr0, rax;

		mov rax, cr4;
		// set OSFXSR and OSXMMEXCPT
		or rax, {(1u32 << 9) | (1u32 << 10) => %i};
		mov cr4, rax;
	}

	asm! {
		[entry => %rax,
			cpu::current().arch.stack.end => %r10, // to rsp
			-1isize => %rbx, // memory mask
			0xDEADDEADu64 => %rdx, // envp
			0xDEADDEADu64 => %rsi, // argv
			0u64 => %rdi, // argc
			0xDEDEDEDEu64 => %rcx,
			0xABABABABABABABABu64 => %r8,
			0xCFCFu64 => %r9,
			0xCFCFCFCFCFCFCFCFu64 => %r11,
			0xBDBDu64 => %r12,
			0xBDBDBDBDu64 => %r13,
			0xBDBDBDBDBDBDBDBDu64 => %r14,
			0xBEEFBEEFBEEFBEEFu64 => %r15]
		mov rsp, r10;
		push r15;
		push r15;
		xor rbp, rbp; // clear rbp so stack backtraces stop
		call rax;
		int 45;
		jmp panic;
	}
}

#[no_mangle]
pub unsafe extern fn panic() {
	panic!("panic() called!");
}
