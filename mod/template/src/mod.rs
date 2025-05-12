//! <Add documentation for your module here>

#![no_std]
#![no_main]

// Do not include kernel symbols in the module
#[no_link]
extern crate kernel;

// Declare the module, with its dependencies
kernel::module!([]);

/// Called on module load
#[unsafe(no_mangle)]
pub extern "C" fn init() -> bool {
	kernel::println!("Hello world!");
	true
}

/// Called on module unload
#[unsafe(no_mangle)]
pub extern "C" fn fini() {
	kernel::println!("Goodbye!");
}
