//! The `pwritev` system call allows to write sparse data on a file descriptor.

use crate::errno::Errno;
use crate::process::iovec::IOVec;
use crate::process::mem_space::ptr::SyscallSlice;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `pwritev` syscall.
#[syscall]
pub fn pwritev(
	fd: c_int,
	iov: SyscallSlice<IOVec>,
	iovcnt: c_int,
	offset: isize,
) -> Result<i32, Errno> {
	super::writev::do_writev(fd, iov, iovcnt, Some(offset), None)
}
