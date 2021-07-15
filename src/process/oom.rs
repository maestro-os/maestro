//! The OOM killer is a procedure which is invoked when the kernel runs out of memory. It kills one
//! or more processes according to a score computed for each of them.

use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::TMutex;

/// Variable telling whether the OOM killer is enabled.
static mut KILLER_ENABLE: Mutex<bool> = Mutex::new(true);

/// Tells whether the OOM killer is enabled.
pub fn is_killer_enabled() -> bool {
	let mutex = unsafe { // Safe because using Mutex
		&mut KILLER_ENABLE
	};
	let guard = mutex.lock();
	*guard.get()
}

/// Enables or disables the OOM killer.
pub fn set_killer_enabled(enable: bool) {
	let mutex = unsafe { // Safe because using Mutex
		&mut KILLER_ENABLE
	};
	let mut guard = mutex.lock();
	*guard.get_mut() = enable;
}

/// Runs the OOM killer.
pub fn kill() {
	if !is_killer_enabled() {
		kernel_panic!("Out of memory");
	}

	// TODO Get the process with the highest OOM score
}
