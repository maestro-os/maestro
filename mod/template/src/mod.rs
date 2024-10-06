//! <Add documentation for your module here>

#![no_std]
#![no_main]

// do not include kernel symbols in the module
#[no_link]
extern crate kernel;

// declare the module, with its dependencies
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
