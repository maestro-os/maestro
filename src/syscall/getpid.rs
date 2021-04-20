/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `getpid` syscall.
pub fn getpid(proc: &mut Process, _regs: &util::Regs) -> u32 {
	proc.get_pid() as _
}
