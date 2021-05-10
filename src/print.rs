//! This file handles macros `print` and `println`.

use crate::tty;
use crate::util::lock::mutex::MutexGuard;

/// Custom writer used to redirect print/println macros to the desired text output.
struct TTYWrite {}

impl core::fmt::Write for TTYWrite {
	fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
		MutexGuard::new(tty::current()).get_mut().write(s);
		Ok(())
	}
}

/// Prints the specified message on the current TTY. This function is meant to be used through
/// `print!` and `println!` macros only.
pub fn _print(args: core::fmt::Arguments) {
	let mut w: TTYWrite = TTYWrite {};
	core::fmt::write(&mut w, args).ok();
}

/// Prints the given formatted string with the given values.
#[allow_internal_unstable(print_internals)]
#[macro_export]
macro_rules! print {
	($($arg:tt)*) => {{
		crate::print::_print(format_args!($($arg)*));
	}};
}

/// Same as `print!`, except it appends a newline at the end.
#[allow_internal_unstable(print_internals, format_args_nl)]
#[macro_export]
macro_rules! println {
	() => (crate::print!("\n"));
	($($arg:tt)*) => {{
		crate::print::_print(format_args_nl!($($arg)*));
	}};
}
