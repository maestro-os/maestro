//! <Add documentation for your module here>

#![no_std]

extern crate kernel;

use kernel::module::version::Version;
use kernel::print;

// hello module, version 1.0.0
kernel::module!("hello", Version::new(1, 0, 0));

/// Called on module load
#[no_mangle]
pub extern "C" fn init() {
	kernel::println!("Hello world!");
}

/// Called on module unload
#[no_mangle]
pub extern "C" fn fini() {
	kernel::println!("Goodbye!");
}
