//! This kernel module implements a driver for the Intel e1000 ethernet controllers.

#![feature(trait_upcasting)]
#![no_std]

extern crate kernel;

mod driver;
mod nic;

use driver::E1000Driver;
use kernel::module::version::Version;

kernel::module!("e1000", Version::new(1, 0, 0), &[]);

/// Called on module load
#[no_mangle]
pub extern "C" fn init() -> bool {
    kernel::device::driver::register(E1000Driver::new()).is_ok()
}

/// Called on module unload
#[no_mangle]
pub extern "C" fn fini() {
    kernel::device::driver::unregister("e1000");
}
