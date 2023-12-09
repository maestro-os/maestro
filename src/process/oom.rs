//! OOM killing is a procedure which is invoked when the kernel runs out of
//! memory.
//!
//! The OOM killer terminates one or more processes according to a score computed for
//! each of them.
//!
//! This is an emergency procedure which is not supposed to be used under normal conditions.

use crate::errno::AllocResult;
use crate::util::lock::Mutex;

/// The maximum number of times the kernel tries to kill a process to retrieve
/// memory.
const MAX_TRIES: u32 = 5;

/// Variable telling whether the OOM killer is enabled.
static KILLER_ENABLE: Mutex<bool> = Mutex::new(true);

/// Tells whether the OOM killer is enabled.
pub fn is_killer_enabled() -> bool {
	*KILLER_ENABLE.lock()
}

/// Enables or disables the OOM killer.
pub fn set_killer_enabled(enable: bool) {
	*KILLER_ENABLE.lock() = enable;
}

/// Runs the OOM killer.
pub fn kill() {
	if !is_killer_enabled() {
		panic!("Out of memory");
	}

	// TODO Get the process with the highest OOM score (ignore init process)
}

/// Executes the given function.
///
/// On fail due to a lack of memory, the function runs the OOM killer, then tries again.
///
/// If the OOM killer is unable to free enough memory, the kernel may panic.
pub fn wrap<T, F: FnMut() -> AllocResult<T>>(mut f: F) -> T {
	for _ in 0..MAX_TRIES {
		if let Ok(r) = f() {
			return r;
		}

		kill();
		// TODO Check if current process has been killed
	}

	panic!("OOM killer is unable to free up space for new allocations!");
}
