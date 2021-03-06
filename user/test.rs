#![allow(improper_ctypes, dead_code)]
#![feature(lang_items,
		   plugin, asm, linkage, const_fn)]
#![no_std]
#![crate_type = "bin"]

#[lang = "eh_unwind_resume"] fn eh_unwind_resume() {}
#[lang = "eh_personality"] fn eh_personality() {}
#[lang = "panic_fmt"] fn panic_fmt() {}

#[lang = "start"]
extern fn start(_main: *const u8, _argc: isize, _argv: *const *const u8) -> isize {
	0
}

fn main() {}
