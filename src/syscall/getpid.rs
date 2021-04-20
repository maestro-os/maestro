/// TODO doc

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `getpid` syscall.
pub fn getpid(proc: &mut Process, _regs: &util::Regs) -> Result<i32, Errno> {
	Ok(proc.get_pid() as _)
}
