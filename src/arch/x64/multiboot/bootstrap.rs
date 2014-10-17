#![no_std]
#![no_builtins]
#![no_main]
#![allow(ctypes)]
#![crate_type = "bin"]
#![feature(asm, globs, lang_items, phase)]
#[phase(plugin)] extern crate assembly;

extern crate core;

use core::prelude::*;
use core::mem::{size_of, size_of_val, uninitialized};

use multiboot::*;

#[lang = "begin_unwind"]
extern fn begin_unwind(args: &core::fmt::Arguments,
                       file: &str,
                       line: uint) -> ! {
    loop {}
}

#[lang = "stack_exhausted"] extern fn stack_exhausted() {}
#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "fail_fmt"] fn fail_fmt() -> ! { loop {} }

type Table = [u64, ..512];

mod multiboot;

#[link_section = ".multiboot"]
pub static HEADER: Header = Header {
    magic: HEADER_MAGIC,
    flags: HEADER_FLAG_PAGE_ALIGN | HEADER_FLAG_MEMORY_INFO,
    checksum: 0 - (HEADER_MAGIC + (HEADER_FLAG_PAGE_ALIGN | HEADER_FLAG_MEMORY_INFO))
}; 

extern {
    static mut pdpt_low: Table;
    static mut pdt_low: Table;
    static mut pt_low: Table;

    static mut pml4t: Table;
    static mut pdpt: Table;
    static mut pdt: Table;
    static mut pt: Table;

    static low_end: u8;
    static kernel_size: u8; // &kernel_size == kernel size in pages
}

#[repr(packed)]
struct Descriptor
{
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8
}

static GDT: [Descriptor, ..3] = [
    Descriptor {limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0},
    Descriptor {limit_low: 0, base_low: 0, base_middle: 0, access: 0b10011000, granularity: 0b00100000, base_high: 0}, // 64-bit code
    Descriptor {limit_low: 0xFFFF, base_low: 0, base_middle: 0, access: 0b10010010, granularity: 0b11001111, base_high: 0}, // data
];

#[repr(packed)]
struct GDT64Pointer {
    limit: u16,
    base: u64
}

static mut GDT64_POINTER: GDT64Pointer = GDT64Pointer {limit: 0, base: 0};

fn offset<T>(ptr: &T) -> u64
{
    return (ptr as *const T) as u64;
}

struct CPUIDResult {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32
}

unsafe fn cpuid(input: u32) -> CPUIDResult
{
    let mut result: CPUIDResult = uninitialized();

    asm! {
        [input => %eax => result.eax, %ebx => result.ebx, %ecx => result.ecx, %edx => result.edx]

        cpuid
    }

    return result;
}

fn halt() -> ! {
    loop {
        unsafe {
            asm! { hlt }
        }
    }
}

fn error(s: &str) -> ! {
    let vga = 0xb8000 as *mut u16;

    unsafe {
        for i in range(0i, 80 * 25) {
            *vga.offset(i) = 0;
        }

        let mut i = 0i;
        for c in s.chars() {
            *vga.offset(82 + i) = c as u16 | (12 << 8);
            i += 1;
        }
    }

    halt();
}

#[no_mangle]
pub unsafe extern fn setup_long_mode(multiboot: u32, magic: u32) {
    if magic != multiboot::MAGIC {
        error("This kernel requires a multiboot compatible loader!");
    }

    if multiboot + 0x1000 > 0x200000 {
        error("Multiboot structure loaded too high in memory");
    }

    // setup the gdt pointer
    GDT64_POINTER.limit = size_of_val(&GDT) as u16 - 1;
    GDT64_POINTER.base = offset(&GDT);
    
    // setup the higher-half
    pml4t[510] = offset(&pml4t) | 3;
    pml4t[511] = offset(&pdpt) | 3;
    pdpt[510] = offset(&pdt) | 3;
    pdt[0] = offset(&pt) | 3;

    let mut physical = offset(&low_end);

    for i in range(0, offset(&kernel_size) as uint) { 
        pt[i] = physical | 3;
        physical += 0x1000;
    }

    // setup the lower-half

    pml4t[0] = offset(&pdpt_low) | 3;
    pdpt_low[0] = offset(&pdt_low) | 3;
    pdt_low[0] = offset(&pt_low) | 3;

    // map the first 2 megabytes

    let mut address = 0;

    for i in range(0, 512) { 
        pt_low[i] = address | 3;
        address += 0x1000;
    }

    if cpuid(0x80000000).eax < 0x80000001 {
        error("Long mode is not supported (no extended flags was found)!");
    }
    
    let long_mode_flag = 1 << 29;
    
    if cpuid(0x80000001).edx & long_mode_flag == 0 {
        error("Long mode is not supported (bit was not set)!");
    }
    
    // load the 64-bit GDT
    asm! {
        lgdt {GDT64_POINTER => %m};
    }
    
    // load PML4T into CR3
    asm! {
        [&pml4t => %eax]

        mov cr3, eax;
    }
    
    asm! {
        // set the long mode bit and nx enable bit
        [0xC0000080u => %ecx, use eax, use edx]

        rdmsr;
        or eax, {1u << 8 => %i};
        wrmsr;

        // enable PAE
        mov eax, cr4;
        or eax, {1u << 5 => %i};
        mov cr4, eax;

        // enable paging
        mov eax, cr0;
        or eax, {1u << 31 => %i};
        mov cr0, eax;
    }
    
    // do a far jump into long mode, pass multiboot information in %ecx
    asm! {
        [multiboot => %ecx, mod attsyntax]

        ljmp {size_of::<Descriptor>() => %i}, $bootstrap.64
    }

    halt();
}
