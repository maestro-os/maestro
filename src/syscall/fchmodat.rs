//! The `fchmodat` system call allows change the permissions on a file.

use super::util;
use crate::errno::Errno;
use crate::file;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

// TODO Check args type
#[syscall]
pub fn fchmodat(
	dirfd: c_int,
	pathname: SyscallString,
	mode: i32,
	_flags: c_int,
) -> Result<i32, Errno> {
	let (file_mutex, uid) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let uid = proc.euid;

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let file_mutex = util::get_file_at(proc, true, dirfd, pathname, 0)?;

		(file_mutex, uid)
	};
	let mut file = file_mutex.lock();

	// Checking permissions
	if uid != file::ROOT_UID && uid != file.get_uid() {
		return Err(errno!(EPERM));
	}

	file.set_permissions(mode as _);
	file.sync()?;

	Ok(0)
}
