use core::prelude::*;
use console;

fn halt() -> ! {
    loop {
        unsafe {
            asm! { hlt }
        }
    }
}

#[no_mangle]
pub extern "C" fn boot_entry() {
    console::cls();
    console::print("Long mode!");
    halt();
} 
