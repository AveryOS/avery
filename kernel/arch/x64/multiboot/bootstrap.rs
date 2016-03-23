#![no_std]
#![crate_type = "staticlib"]
#![feature(asm, lang_items, plugin)]
#![plugin(assembly)]
#![plugin(clippy)]
#![warn(cast_possible_truncation, cast_possible_wrap,
        cast_precision_loss, cast_sign_loss)]

extern crate rlibc;

use core::mem::{size_of_val, uninitialized};
use core::fmt::{Write, Arguments, Error};

use multiboot::*;

static mut VGA: *mut u16 = 0xb8000 as *mut u16;

struct ScreenWriter(isize);

impl Write for ScreenWriter {
	fn write_str(&mut self, s: &str) -> Result<(), Error> {
		for c in s.chars() {
			unsafe {
				*VGA.offset(82 + self.0) = c as u16 | (12 << 8);
			}
			self.0 += 1;
		}

		Ok(())
	}
}

macro_rules! error {
	($($arg:tt)*) => (
		error_args(format_args!($($arg)*))
	)
}

fn error_args(args: Arguments) -> ! {
	unsafe {
		for i in 0isize..(80 * 25) {
			*VGA.offset(i) = 0;
		}
		assert!(ScreenWriter(0).write_fmt(args).is_ok());
		loop {
			asm! { cli; hlt }
		}
	}
}

#[lang = "eh_unwind_resume"] fn eh_unwind_resume() {}
#[lang = "eh_personality"] fn eh_personality() {}
#[lang = "panic_fmt"] extern fn panic_fmt(fmt: Arguments, file: &'static str, line: u32) -> ! {
	error!("Panic: {} - Loc: {}:{}", fmt, file, line);
}

type Table = [u64; 512];

mod multiboot;

#[link_section = ".multiboot"]
pub static HEADER: Header = Header {
	magic: HEADER_MAGIC,
	flags: HEADER_FLAG_PAGE_ALIGN | HEADER_FLAG_MEMORY_INFO,
	checksum: -((HEADER_MAGIC + (HEADER_FLAG_PAGE_ALIGN | HEADER_FLAG_MEMORY_INFO)) as i32) as u32
};

extern {
	static mut pdpt_low: Table;
	static mut pdt_low: Table;
	static mut pt_low: Table;

	static mut pml4t: Table;
	static mut pdpt: Table;
	static mut pdt: Table;
	static mut pts: [Table; 16];

	static low_end: u8;
	static kernel_size: u8; // &kernel_size == kernel size in pages
}

#[repr(packed)]
#[allow(dead_code)]
struct Descriptor {
	limit_low: u16,
	base_low: u16,
	base_middle: u8,
	access: u8,
	granularity: u8,
	base_high: u8
}

static GDT: [Descriptor; 3] = [
	Descriptor {limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0},
	Descriptor {limit_low: 0, base_low: 0, base_middle: 0, access: 0b10011000, granularity: 0b00100000, base_high: 0}, // 64-bit code
	Descriptor {limit_low: 0xFFFF, base_low: 0, base_middle: 0, access: 0b10010010, granularity: 0b11001111, base_high: 0}, // data
];

#[repr(packed)]
#[allow(dead_code)]
struct GDT64Pointer {
	limit: u16,
	base: u64
}

fn offset<T>(ptr: &'static T) -> u64 {
	ptr as *const T as u64
}

struct CPUIDResult {
	eax: u32,
	ebx: u32,
	ecx: u32,
	edx: u32
}

unsafe fn cpuid(input: u32) -> CPUIDResult {
	let mut result: CPUIDResult = uninitialized();

	asm! {
		[input => %eax => result.eax, %ebx => result.ebx, %ecx => result.ecx, %edx => result.edx]

		cpuid
	}

	result
}

#[no_mangle]
pub unsafe extern fn setup_long_mode(multiboot: u32, magic: u32) {
	if magic != multiboot::MAGIC {
		error!("This kernel requires a multiboot compatible loader!");
	}

	if multiboot + 0x1000 > 0x200000 {
		error!("Multiboot structure loaded too high in memory");
	}

	// setup the higher-half
	pml4t[510] = offset(&pml4t) | 3;
	pml4t[511] = offset(&pdpt) | 3;
	pdpt[510] = offset(&pdt) | 3;

	let mut physical = offset(&low_end);

	for i in 0..(offset(&kernel_size) as usize) {
		let pt_i = i % 512;
		let pdt_i = i / 512;
		pdt[pdt_i] = offset(&pts[pdt_i]) | 3;
		pts[pdt_i][pt_i] = physical | 3;
		physical += 0x1000;
	}

	// setup the lower-half

	pml4t[0] = offset(&pdpt_low) | 3;
	pdpt_low[0] = offset(&pdt_low) | 3;
	pdt_low[0] = offset(&pt_low) | 3;

	// map the first 2 megabytes

	let mut address = 0;

	for i in 0usize..512 {
		pt_low[i] = address | 3;
		address += 0x1000;
	}

	if cpuid(0x80000000).eax < 0x80000001 {
		error!("Long mode is not supported (no extended flags was found)!");
	}

	let long_mode_flag = 1 << 29;

	if cpuid(0x80000001).edx & long_mode_flag == 0 {
		error!("Long mode is not supported (bit was not set)!");
	}

	let gdt_ptr = GDT64Pointer {
		limit: size_of_val(&GDT) as u16 - 1,
		base: offset(&GDT)
	};

	// load the 64-bit GDT
	asm! {
		lgdt {&gdt_ptr => %*m};
	}

	// load PML4T into CR3
	asm! {
		[&pml4t => %eax]

		mov cr3, eax;
	}

	asm! {
		// set the long mode bit and nx enable bit
		[0xC0000080usize => %ecx, use eax, use edx]

		rdmsr;
		or eax, {((1usize << 8) | (1usize << 11)) => %i};
		wrmsr;

		// enable PAE
		mov eax, cr4;
		or eax, {1usize << 5 => %i};
		mov cr4, eax;

		// enable paging
		mov eax, cr0;
		or eax, {1usize << 31 => %i};
		mov cr0, eax;
	}

	// do a far jump into long mode, pass multiboot information in %ecx
	asm! {
		[multiboot => %ecx, mod attsyntax]

		ljmp $8, $bootstrap.64
	}

	error!("Couldn't jump into long mode");
}
