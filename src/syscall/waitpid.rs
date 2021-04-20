/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `waitpid` syscall.
pub fn waitpid(_proc: &mut Process, _regs: &util::Regs) -> u32 {
	// TODO
	0
}
