//! <Add documentation for your module here>

#![cfg_attr(not(maestro_builtin), no_std)]
#![cfg_attr(not(maestro_builtin), no_main)]

// Do not include kernel symbols in the module
#[cfg(not(maestro_builtin))]
#[no_link]
extern crate kernel;

// Declare the module, with its dependencies
kernel::module!([]);

/// Called on module load
#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn init() -> bool {
	kernel::println!("Hello world!");
	true
}

/// Called on module unload
#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn fini() {
	kernel::println!("Goodbye!");
}
