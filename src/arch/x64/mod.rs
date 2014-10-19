#[cfg(multiboot)]
pub mod multiboot;

pub mod console {
	pub use super::vga::{cls, putc};
}

pub fn halt() -> ! {
    loop {
        unsafe {
            asm! { hlt }
        }
    }
}

unsafe fn outb(port: u16, value: u8)
{
	asm! {
		out {port => %dx}, {value => %al}
	}
}

mod vga;