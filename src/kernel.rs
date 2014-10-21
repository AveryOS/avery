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

#[no_mangle]
pub extern fn ap_entry() {
} 

fn kernel(info: &mut params::Info) {
	panic!("Bored");
}
