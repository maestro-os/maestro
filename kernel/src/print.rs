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

//! Implementation of printing/logging macros.
//!
//! Unlike the print macros from Rust's standard library, these are used to log informations
//! instead of only printing.
//!
//! Printing can be silenced at boot using the `-silent` command line argument, but logs remain in
//! memory.

use crate::logger;
use core::fmt;

/// Prints/logs the given message.
///
/// This function is meant to be used through [`print!`] and [`println!`] macros only.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
	fmt::write(&mut *logger::BUF.lock(), args).ok();
}

/// DEBUG: writes only to the serial port (not the VGA TTY), so the framebuffer stays clean.
struct SerialOnly;
impl fmt::Write for SerialOnly {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		crate::device::serial::PORTS[0].lock().write(s.as_bytes());
		Ok(())
	}
}
#[doc(hidden)]
pub fn _dbg_serial(args: fmt::Arguments) {
	fmt::write(&mut SerialOnly, args).ok();
}

/// DEBUG: like `println!` but serial-only.
#[macro_export]
#[allow_internal_unstable(print_internals, format_args_nl)]
macro_rules! dbgs {
	($($arg:tt)*) => {{
		$crate::print::_dbg_serial(format_args_nl!($($arg)*));
	}};
}

/// Prints the given formatted string with the given values.
#[allow_internal_unstable(print_internals)]
#[macro_export]
macro_rules! print {
	($($arg:tt)*) => {{
		$crate::print::_print(format_args!($($arg)*));
	}};
}

/// Same as [`crate::print!`], except it appends a newline at the end.
#[allow_internal_unstable(print_internals, format_args_nl)]
#[macro_export]
macro_rules! println {
	() => ($crate::print!("\n"));
	($($arg:tt)*) => {{
		$crate::print::_print(format_args_nl!($($arg)*));
	}};
}
