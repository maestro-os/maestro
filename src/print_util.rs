/// This file contains utilities for the `print` and `println` macros.

use crate::tty;
use crate::util::lock::MutexGuard;

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
