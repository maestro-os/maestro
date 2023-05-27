//! The `mmap2` system call is similar to `mmap`, except it takes a file offset
//! in pages.

use super::mmap;
use crate::errno::Errno;
use core::ffi::c_int;
use core::ffi::c_void;
use macros::syscall;

// TODO Check last argument type
#[syscall]
pub fn mmap2(
	addr: *mut c_void,
	length: usize,
	prot: c_int,
	flags: c_int,
	fd: c_int,
	offset: u64,
) -> Result<i32, Errno> {
	mmap::do_mmap(addr, length, prot, flags, fd, offset * 4096)
}
