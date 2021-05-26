//! This modules handles kernel logging.
//! If the logger is silent, it will not print the logs on the screen but it will keep it in memory
//! anyways.

use crate::tty;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;

/// Tells whether the logger is silent.
static mut SILENT: Mutex<bool> = Mutex::new(false);

/// Tells whether the logger is silent.
pub fn is_silent() -> bool {
	let mutex = unsafe { // Safe because using Mutex
		&mut SILENT
	};
	let guard = MutexGuard::new(mutex);
	*guard.get()
}

/// Sets the logger silent or not.
pub fn set_silent(silent: bool) {
	let mutex = unsafe { // Safe because using Mutex
		&mut SILENT
	};
	let mut guard = MutexGuard::new(mutex);
	*guard.get_mut() = silent;
}

/// Custom writer used to redirect print/println macros to the logger.
pub struct LoggerWrite {}

impl core::fmt::Write for LoggerWrite {
	fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
		if !is_silent() {
			MutexGuard::new(tty::current()).get_mut().write(s);
		}

		// TODO Keep in memory

		Ok(())
	}
}
