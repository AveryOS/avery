use cpu::CPUVec;
use util::FixVec;

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

pub fn halt() -> ! {
    loop {
        unsafe {
            asm! { hlt }
        }
    }
}

unsafe fn write_msr(reg: u32, value: usize)
{
	asm! {
		[value => %eax, value >> 32 => %edx, reg => %ecx]

		wrmsr
	}
}

unsafe fn outb(port: u16, value: u8)
{
	asm! {
		out {port => %dx}, {value => %al}
	}
}

mod vga;
mod acpi;

pub mod segments;
pub mod interrupts;
pub mod cpu;
pub mod memory;

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
	cpu::map_local_page_tables(cpu::bsp());

	let mut cpu_info = CPUVec::new();
	acpi::initialize(&mut cpu_info);
}
