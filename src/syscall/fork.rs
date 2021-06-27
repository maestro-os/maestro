//! TODO doc

use crate::errno::Errno;
use crate::process::Process;
use crate::util::lock::mutex::TMutex;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(proc: &mut Process, _regs: &util::Regs) -> Result<i32, Errno> {
	let mut mutex = proc.fork()?;
	let mut guard = mutex.lock();
	let new_proc = guard.get_mut();

	Ok(new_proc.get_pid() as _)
}
