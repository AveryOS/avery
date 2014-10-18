#![no_std]
#![no_main]
#![allow(ctypes)]
#![feature(globs, lang_items, phase, macro_rules, asm)]
#[phase(plugin)] extern crate assembly;

extern crate core;
extern crate rlibc;

use core::prelude::*;
use core::mem;

#[path = "arch/x64/mod.rs"]
pub mod arch;

#[lang = "begin_unwind"]
extern fn begin_unwind(args: &core::fmt::Arguments,
                       file: &str,
                       line: uint) -> ! {
    loop {}
}

#[lang = "stack_exhausted"] extern fn stack_exhausted() {}
#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "fail_fmt"] fn fail_fmt() -> ! { loop {} }

#[no_mangle]
pub extern fn ap_entry() {
} 
