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

use crate::{debug, power};
use core::{
	any::type_name,
	sync::{atomic, atomic::AtomicBool},
};

/// Boolean value telling whether selftesting is running.
static RUNNING: AtomicBool = AtomicBool::new(false);

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
		crate::print!("test {name} ... ");
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
	RUNNING.store(true, atomic::Ordering::Relaxed);
	for test in tests {
		test.run();
	}
	RUNNING.store(false, atomic::Ordering::Relaxed);
	crate::println!("No more tests to run");
	#[cfg(config_debug_qemu)]
	debug::qemu::exit(debug::qemu::SUCCESS);
	power::halt();
}

/// Tells whether selftesting is running.
pub fn is_running() -> bool {
	RUNNING.load(atomic::Ordering::Relaxed)
}
