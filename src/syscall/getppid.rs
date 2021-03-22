/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `getppid` syscall.
pub fn getppid(_regs: &util::Regs) -> u32 {
	Process::get_current().unwrap().get_parent_pid() as _
}
