#![no_main]
#![allow(improper_ctypes)]
#![feature(lang_items, plugin, asm, negate_unsigned, core_slice_ext, core_str_ext)]
#![plugin(assembly)]

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
		arch::memory::initialize_initial(result);
		memory::physical::initialize();
		memory::initialize();
	}

	panic!("Bored");
}

fn kernel() {
	panic!("Bored");
}
