//! Implementation of printing/logging macros.
//!
//! Unlike the print macros from Rust's standard library, these are used to log informations
//! instead of only printing.
//!
//! Printing can be silenced at boot using the `-silent` command line argument, but logs remain in
//! memory.

use crate::logger::LOGGER;
use core::fmt;

/// Prints/logs the given message.
///
/// This function is meant to be used through [`print!`] and [`println!`] macros only.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
	let mut logger = LOGGER.lock();
	fmt::write(&mut *logger, args).ok();
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
