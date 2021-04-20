/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `getppid` syscall.
pub fn getppid(proc: &mut Process, _regs: &util::Regs) -> u32 {
	proc.get_parent_pid() as _
}
