#![allow(improper_ctypes, dead_code)]
#![feature(lang_items,
		   plugin, asm, negate_unsigned, core_intrinsics, linkage, const_fn)]
#![no_std]
#![crate_type = "bin"]

#[lang = "eh_personality"] fn eh_personality() {}
#[lang = "panic_fmt"] fn panic_fmt() {}

#[lang = "start"]
extern fn start(main: *const u8, argc: isize, argv: *const *const u8) -> isize {
	0
}

fn main() {}