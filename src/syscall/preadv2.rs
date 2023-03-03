//! The `readv` system call allows to read from file descriptor and write it into a sparse buffer.

use crate::errno::Errno;
use crate::process::iovec::IOVec;
use crate::process::mem_space::ptr::SyscallSlice;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn preadv2(
	fd: c_int,
	iov: SyscallSlice<IOVec>,
	iovcnt: c_int,
	offset: isize,
	flags: c_int,
) -> Result<i32, Errno> {
	super::readv::do_readv(fd, iov, iovcnt, Some(offset), Some(flags))
}
