//! The `unlink` system call deletes the given file from its filesystem. If no link remain to the
//! inode, the function also removes the inode.

use crate::errno::Errno;
use crate::process::Regs;

/// The implementation of the `unlink` syscall.
pub fn unlink(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
