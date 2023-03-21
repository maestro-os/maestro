//! The `fcntl64` syscall call allows to manipulate a file descriptor.

use crate::errno::Errno;
use core::ffi::c_int;
use core::ffi::c_void;
use macros::syscall;

#[syscall]
pub fn fcntl64(fd: c_int, cmd: c_int, arg: *mut c_void) -> Result<i32, Errno> {
	super::fcntl::do_fcntl(fd, cmd, arg, true)
}
