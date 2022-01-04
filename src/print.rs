//! This file handles macros `print` and `println`. Unlink the standard print operations, these are
//! used to log kernel informations. They can be silenced at boot using the `-silent` command line
//! argument but they will be kept in the logger anyways.

use crate::logger;

/// Prints the specified message on the current TTY. This function is meant to be used through
/// `print!` and `println!` macros only.
pub fn _print(args: core::fmt::Arguments) {
	let mutex = logger::get();
	let mut guard = mutex.lock();
	core::fmt::write(guard.get_mut(), args).ok();
}

/// Prints the given formatted string with the given values.
#[allow_internal_unstable(print_internals)]
#[macro_export]
macro_rules! print {
	($($arg:tt)*) => {{
		$crate::print::_print(format_args!($($arg)*));
	}};
}

/// Same as `print!`, except it appends a newline at the end.
#[allow_internal_unstable(print_internals, format_args_nl)]
#[macro_export]
macro_rules! println {
	() => ($crate::print!("\n"));
	($($arg:tt)*) => {{
		$crate::print::_print(format_args_nl!($($arg)*));
	}};
}
