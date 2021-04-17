/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `getpid` syscall.
pub fn getpid(_regs: &util::Regs) -> u32 {
	Process::get_current().unwrap().lock().get().get_pid() as _
}
