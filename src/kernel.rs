#![no_main]
#![allow(improper_ctypes, dead_code)]
#![feature(core, core_char_ext, lang_items,
           plugin, asm, negate_unsigned, core_slice_ext,
		   core_str_ext, core_intrinsics)]
#![plugin(assembly)]

extern crate core;

extern crate rlibc;

#[macro_use]
mod util;

#[macro_use]
mod console;

mod params;


#[path = "arch/x64/mod.rs"]
pub mod arch;

mod cpu;

mod memory;

#[no_mangle]
pub extern fn ap_entry() {
}

fn init(info: &mut params::Info) {
	unsafe {
		arch::initialize_basic();
		let result = memory::initial::initialize_physical(info);
		arch::memory::initialize_initial(&result);
		memory::physical::initialize(&result);
		memory::initialize();
		arch::initialize();
	}

	panic!("Bored");
}

fn kernel() {
	panic!("Bored");
}
