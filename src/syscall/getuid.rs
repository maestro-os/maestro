//! The `getuid` syscall returns the UID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `getuid` syscall.
pub fn getuid(proc: &mut Process, _: &util::Regs) -> Result<i32, Errno> {
	Ok(proc.get_uid() as _)
}
