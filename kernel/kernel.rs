#![allow(improper_ctypes, dead_code)]
#![feature(lang_items, alloc, collections,
		   plugin, asm, core_intrinsics, linkage, const_fn,
		   allocator)]
#![plugin(assembly)]
#![plugin(clippy)]
#![crate_type = "staticlib"]

// clippy lints
#![allow(cyclomatic_complexity, similar_names, if_not_else,
	     needless_lifetimes, len_without_is_empty, new_without_default)]
#![warn(cast_possible_truncation, cast_possible_wrap,
        cast_precision_loss, cast_sign_loss)]

extern crate core;
extern crate allocator;

extern crate rlibc;
extern crate elfloader;
extern crate alloc;
extern crate collections;

#[macro_use]
mod util;

#[macro_use]
mod console;

mod params;

mod process;

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
