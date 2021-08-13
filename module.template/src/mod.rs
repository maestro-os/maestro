//! <Add documentation for your module here>

#![no_std]

use abi::print;

// This function is called on module intialization
#[no_mangle]
pub extern "C" fn init() {
	abi::println!("Hello world!");
}

// This function is called on module destruction
#[no_mangle]
pub extern "C" fn fini() {
	abi::println!("Goodbye!");
}
