use core::prelude::*;

fn halt() -> ! {
    loop {
        unsafe {
            asm! { hlt }
        }
    }
}

fn error(s: &str) -> ! {
    let vga = 0xb8000 as *mut u16;

    unsafe {
        for i in range(0i, 80 * 25) {
            *vga.offset(i) = 0;
        }

        let mut i = 0i;
        for c in s.chars() {
            *vga.offset(82 + i) = c as u16 | (12 << 8);
            i += 1;
        }
    }

    halt();
}

#[no_mangle]
pub extern fn boot_entry() {
	error("Long mode!");
} 
