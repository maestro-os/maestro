//! This kernel module implements a driver for the Intel e1000 ethernet controllers.

#![no_std]
#![no_main]
#![feature(likely_unlikely)]
#![allow(unused)] // TODO remove

#[no_link]
extern crate kernel;

// FIXME
//mod driver;
mod nic;

kernel::module!([]);

/// Called on module load
#[unsafe(no_mangle)]
pub extern "C" fn init() -> bool {
	// FIXME
	//kernel::device::driver::register(E1000Driver::new()).is_ok()
	todo!()
}

/// Called on module unload
#[unsafe(no_mangle)]
pub extern "C" fn fini() {
	// FIXME
	//kernel::device::driver::unregister("e1000");
}
