//! TODO doc

use crate::errno::Errno;
use crate::util;

/// The implementation of the `read` syscall.
pub fn read(_regs: &util::Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
