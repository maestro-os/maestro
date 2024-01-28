//! The `creat` system call allows to create and open a file.

use super::open;
use crate::{errno::Errno, file::open_file, process::mem_space::ptr::SyscallString};
use core::ffi::c_int;
use macros::syscall;

// TODO Check args type
#[syscall]
pub fn creat(pathname: SyscallString, mode: c_int) -> Result<i32, Errno> {
	let flags = open_file::O_CREAT | open_file::O_WRONLY | open_file::O_TRUNC;
	open::open_(pathname, flags, mode as _)
}
