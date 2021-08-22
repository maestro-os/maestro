//! The OOM killer is a procedure which is invoked when the kernel runs out of memory. It kills one
//! or more processes according to a score computed for each of them.

use crate::process::Errno;
use crate::util::lock::mutex::Mutex;

/// The maximum number of times the kernel tries to kill a process to retrieve memory.
const MAX_TRIES: u32 = 5;

/// Variable telling whether the OOM killer is enabled.
static mut KILLER_ENABLE: Mutex<bool> = Mutex::new(true);

/// Tells whether the OOM killer is enabled.
pub fn is_killer_enabled() -> bool {
	let mutex = unsafe { // Safe because using Mutex
		&mut KILLER_ENABLE
	};
	let guard = mutex.lock(true);
	*guard.get()
}

/// Enables or disables the OOM killer.
pub fn set_killer_enabled(enable: bool) {
	let mutex = unsafe { // Safe because using Mutex
		&mut KILLER_ENABLE
	};
	let mut guard = mutex.lock(true);
	*guard.get_mut() = enable;
}

/// Runs the OOM killer.
pub fn kill() {
	if !is_killer_enabled() {
		kernel_panic!("Out of memory");
	}

	// TODO Get the process with the highest OOM score
}

/// Executes the given function. On fail due to a lack of memory, the function runs the OOM killer,
/// then tries again. If the OOM killer is unable to free enough memory, the kernel may panic.
pub fn wrap<T, F: FnMut() -> Result<T, Errno>>(mut f: F) -> T {
	for _ in 0..MAX_TRIES {
		if let Ok(r) = f() {
			return r;
		}

		kill();
		// TODO Check if current process has been killed
	}

	crate::kernel_panic!("OOM killer is unable to free up space for new allocations!");
}
