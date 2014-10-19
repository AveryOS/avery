use arch;

#[no_mangle]
pub extern "C" fn boot_entry() {
    arch::console::cls();
    panic!("Long mode!");
} 
