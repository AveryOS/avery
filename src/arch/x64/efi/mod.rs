use core;
use params;

#[no_mangle]
pub extern "C" fn boot_entry(params: &mut params::Info) {
    let mut params = *params;

	::kernel(&mut params);
} 
