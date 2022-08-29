//! The `chroot` system call allows to virtually redefine the system's root for the current
//! process.

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `chroot` syscall.
pub fn chroot(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
