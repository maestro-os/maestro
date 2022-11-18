//! The `break` system call is not implemented.

use crate::errno;
use crate::errno::Errno;

/// The implementation of the `break` syscall.
pub fn r#break() -> Result<i32, Errno> {
	Err(errno!(ENOSYS))
}
