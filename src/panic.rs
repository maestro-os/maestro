//! This module implements kernel panics handling.
//!
//! A kernel panic occurs when an error is raised that the kernel cannot recover
//! from. This is an undesirable state which requires to reboot the host
//! machine.

use crate::{cpu, logger, power};
use core::panic::PanicInfo;

/// Called on Rust panic.
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	crate::cli!();
	logger::LOGGER.lock().silent = false;

	#[cfg(test)]
	{
		use crate::selftest;
		if selftest::is_running() {
			crate::println!("FAILED\n");
			crate::println!("Error: {panic_info}\n");

			#[cfg(config_debug_qemu)]
			selftest::qemu::exit(selftest::qemu::FAILURE);
			power::halt();
		}
	}

	crate::println!("--- KERNEL PANIC ---\n");
	crate::println!("Kernel has been forced to halt due to internal problem, sorry :/");
	if let Some(msg) = panic_info.message() {
		crate::print!("Reason: {msg}");
	}
	if let Some(loc) = panic_info.location() {
		crate::println!(" (location: {loc})");
	} else {
		crate::println!();
	}
	crate::println!(
		"If you believe this is a bug on the kernel side, please feel free to report it."
	);

	let cr2 = unsafe { cpu::cr2_get() };
	crate::println!("cr2: {cr2:p}\n");

	#[cfg(config_debug_debug)]
	{
		use crate::debug;
		use core::ffi::c_void;
		use core::ptr::null_mut;

		crate::println!("--- Callstack ---");
		unsafe {
			let ebp = crate::register_get!("ebp") as *mut _;
			let mut callstack: [*mut c_void; 8] = [null_mut::<c_void>(); 8];
			debug::get_callstack(ebp, &mut callstack);
			debug::print_callstack(&callstack);
		}
	}

	power::halt();
}

// TODO check whether this can be removed since the kernel uses panic=abort
/// Function that is required to be implemented by the Rust compiler and is used
/// only when panicking.
#[lang = "eh_personality"]
fn eh_personality() {}
