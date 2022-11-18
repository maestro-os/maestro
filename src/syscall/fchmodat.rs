//! The `fchmodat` system call allows change the permissions on a file.

use super::util;
use crate::errno::Errno;
use crate::file;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

// TODO Check args type
/// The implementation of the `fchmodat` syscall.
#[syscall]
pub fn fchmodat(
	dirfd: c_int,
	pathname: SyscallString,
	mode: i32,
	_flags: c_int,
) -> Result<i32, Errno> {
	let (file_mutex, uid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let uid = proc.get_euid();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let file_mutex = util::get_file_at(guard, true, dirfd, pathname, 0)?;

		(file_mutex, uid)
	};
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();

	// Checking permissions
	if uid != file::ROOT_UID && uid != file.get_uid() {
		return Err(errno!(EPERM));
	}

	file.set_permissions(mode as _);
	file.sync()?;

	Ok(0)
}
