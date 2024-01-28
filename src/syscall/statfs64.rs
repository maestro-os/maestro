//! The `statfs64` system call returns information about a mounted file system.

use super::statfs::do_statfs;
use crate::errno::Errno;
use crate::file::fs::Statfs;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use macros::syscall;

// TODO Check args types
#[syscall]
pub fn statfs64(path: SyscallString, _sz: usize, buf: SyscallPtr<Statfs>) -> Result<i32, Errno> {
	// TODO Use `sz`
	do_statfs(path, buf)
}
