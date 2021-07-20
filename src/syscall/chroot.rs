//! TODO doc

use crate::errno::Errno;
use crate::util;

/// The implementation of the `chroot` syscall.
pub fn chroot(_regs: &util::Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
