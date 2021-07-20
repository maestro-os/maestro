//! TODO doc

use crate::errno::Errno;
use crate::util;

/// The implementation of the `unlink` syscall.
pub fn unlink(_regs: &util::Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
