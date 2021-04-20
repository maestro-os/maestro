/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `chroot` syscall.
pub fn chroot(_proc: &mut Process, _regs: &util::Regs) -> u32 {
	// TODO
	0
}
