use params;

#[no_mangle]
pub extern "C" fn boot_entry(params: &mut params::Info) {
	::init(params);
}
