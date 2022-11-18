//! The `fstat64` system call allows get the status of a file.

use crate::errno::Errno;
use macros::syscall;

/// The implementation of the `fstat64` syscall.
#[syscall]
pub fn fstat64() -> Result<i32, Errno> {
	// TODO
	todo!();
}
