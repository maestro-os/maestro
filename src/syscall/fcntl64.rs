//! The `fcntl64` syscall call allows to manipulate a file descriptor.

use core::ffi::c_int;
use crate::errno::Errno;
use core::ffi::c_void;
use macros::syscall;

/// The implementation of the `fcntl64` syscall.
#[syscall]
pub fn fcntl64(fd: c_int, cmd: c_int, arg: *mut c_void) -> Result<i32, Errno> {
	super::fcntl::do_fcntl(fd, cmd, arg, true)
}
