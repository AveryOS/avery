#![no_main]
#![allow(ctypes)]
#![feature(globs, lang_items, phase, macro_rules, asm)]
#[phase(plugin)] extern crate assembly;

//extern crate std;
#[phase(plugin, link)] extern crate core;
extern crate rlibc;

#[macro_escape]
mod util;

#[macro_escape]
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
		memory::initial::initialize_physical(info);
	}

	panic!("Bored");
}

fn kernel() {
	panic!("Bored");
}
