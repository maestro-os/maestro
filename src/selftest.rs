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

//! Selftesting are unit tests or integration tests that run on the kernel itself.
//!
//! # Issues
//!
//! Since the kernel cannot reset itself between each test, this method of testing might not be
//! entirely trustable because a test might corrupt the environment for the next tests, which might
//! make them pass even though they should not. Even if this scenario is unlikely, this remains a
//! concern since the kernel has to be as reliable as possible.

use crate::power;
use core::any::type_name;

/// Boolean value telling whether selftesting is running.
static mut RUNNING: bool = false;

/// This module contains utilities to manipulate QEMU for testing.
#[cfg(config_debug_qemu)]
pub mod qemu {
	use crate::io;

	/// The port used to trigger QEMU emulator exit with the given exit code.
	const EXIT_PORT: u16 = 0xf4;

	/// QEMU exit code for success.
	pub const SUCCESS: u32 = 0x10;
	/// QEMU exit code for failure.
	pub const FAILURE: u32 = 0x11;

	/// Exits QEMU with the given status.
	pub fn exit(status: u32) {
		unsafe {
			io::outl(EXIT_PORT, status);
		}
	}
}

/// Trait for any testable feature.
pub trait Testable {
	/// Function called to run the corresponding test.
	fn run(&self);
}

impl<T> Testable for T
where
	T: Fn(),
{
	fn run(&self) {
		let name = type_name::<T>();
		crate::print!("test {} ... ", name);

		self();

		crate::println!("ok");
	}
}

/// The test runner for the kernel.
///
/// This function runs every tests for the kernel and halts the kernel or exits the emulator if
/// possible.
pub fn runner(tests: &[&dyn Testable]) {
	crate::println!("Running {} tests", tests.len());

	unsafe {
		// Safe because the function is called by only one thread
		RUNNING = true;
	}

	for test in tests {
		test.run();
	}

	unsafe {
		// Safe because the function is called by only one thread
		RUNNING = false;
	}

	crate::println!("No more tests to run");

	#[cfg(config_debug_qemu)]
	qemu::exit(qemu::SUCCESS);
	power::halt();
}

/// Tells whether selftesting is running.
pub fn is_running() -> bool {
	unsafe {
		// Safe because the function is called by only one thread
		RUNNING
	}
}
