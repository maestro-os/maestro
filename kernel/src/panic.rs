/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module implements kernel panics handling.
//!
//! A kernel panic occurs when an error is raised that the kernel cannot recover
//! from. This is an undesirable state which requires to reboot the host
//! machine.

use crate::{logger, memory::VirtAddr, power, register_get};
use core::panic::PanicInfo;
use utils::interrupt::cli;

/// Called on Rust panic.
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	cli();
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
	crate::print!("Reason: {}", panic_info.message());
	if let Some(loc) = panic_info.location() {
		crate::println!(" (location: {loc})");
	} else {
		crate::println!();
	}
	crate::println!(
		"If you believe this is a bug on the kernel side, please feel free to report it."
	);

	crate::println!("cr2: {:?}\n", VirtAddr(register_get!("cr2")));

	#[cfg(debug_assertions)]
	{
		use crate::debug;
		use core::ptr;

		crate::println!("--- Callstack ---");
		#[cfg(target_arch = "x86")]
		let frame = register_get!("ebp");
		#[cfg(target_arch = "x86_64")]
		let frame = register_get!("rbp");
		let ebp = ptr::with_exposed_provenance(frame);
		let mut callstack: [VirtAddr; 8] = [VirtAddr::default(); 8];
		unsafe {
			debug::get_callstack(ebp, &mut callstack);
		}
		debug::print_callstack(&callstack);
	}

	power::halt();
}

// TODO check whether this can be removed since the kernel uses panic=abort
/// Function that is required to be implemented by the Rust compiler and is used
/// only when panicking.
#[lang = "eh_personality"]
fn eh_personality() {}
