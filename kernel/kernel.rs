#![allow(improper_ctypes, dead_code)]
#![feature(lang_items,
		   plugin, asm, core_intrinsics, linkage, const_fn,
		   allocator)]
#![plugin(assembly)]
#![crate_type = "staticlib"]

extern crate core;
extern crate allocator;

extern crate rlibc;
extern crate elfloader;

#[macro_use]
mod util;

#[macro_use]
mod console;

mod params;

mod spin;

#[path = "arch/x64/mod.rs"]
pub mod arch;

mod cpu;

pub mod memory;

fn init(info: &mut params::Info) {
	unsafe {
		cpu::initialize_basic();
		arch::initialize_basic();
		let result = memory::initial::initialize_physical(info);
		arch::memory::initialize_initial(&result);
		memory::physical::initialize(&result);
		memory::initialize();
		arch::initialize();
	}

	panic!("Bored");
}
