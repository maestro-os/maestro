//! The `creat` system call allows to create and open a file.

use core::ffi::c_int;
use super::open;
use crate::errno::Errno;
use crate::file::open_file;
use crate::process::mem_space::ptr::SyscallString;
use macros::syscall;

// TODO Check args type
/// The implementation of the `creat` syscall.
#[syscall]
pub fn creat(pathname: SyscallString, mode: c_int) -> Result<i32, Errno> {
	let flags = open_file::O_CREAT | open_file::O_WRONLY | open_file::O_TRUNC;
	open::open_(pathname, flags, mode as _)
}
