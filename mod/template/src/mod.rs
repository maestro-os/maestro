//! <Add documentation for your module here>

#![no_std]
#![no_main]

// hello module, version 1.0.0
kernel::module!([]);

/// Called on module load
#[no_mangle]
pub extern "C" fn init() -> bool {
	kernel::println!("Hello world!");
	true
}

/// Called on module unload
#[no_mangle]
pub extern "C" fn fini() {
	kernel::println!("Goodbye!");
}
