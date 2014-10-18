extern crate core;

use core::prelude::*;

#[cfg(multiboot)]
pub mod multiboot;

pub mod console {
	pub use super::vga::{print, cls};
}

unsafe fn outb(port: u16, value: u8)
{
	asm! {
		out {port => %dx}, {value => %al}
	}
}

mod vga;