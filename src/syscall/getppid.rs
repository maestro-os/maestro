/// TODO doc

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `getppid` syscall.
pub fn getppid(proc: &mut Process, _regs: &util::Regs) -> Result<i32, Errno> {
	Ok(proc.get_parent_pid() as _)
}
