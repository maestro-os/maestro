//! The `getgid` syscall returns the GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `getgid` syscall.
pub fn getgid(proc: &mut Process, _: &util::Regs) -> Result<i32, Errno> {
	Ok(proc.get_gid() as _)
}
