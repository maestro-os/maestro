//! The `chmod` system call allows change the permissions on a file.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `chmod` syscall.
#[syscall]
pub fn chmod(pathname: SyscallString, mode: c_int) -> Result<i32, Errno> {
	let (path, uid, gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let path = Path::from_str(path, true)?;
		let path = super::util::get_absolute_path(proc, path)?;
		(path, proc.get_euid(), proc.get_egid())
	};

	let file_mutex = {
		let mutex = vfs::get();
		let guard = mutex.lock();
		let vfs = guard.get_mut().as_mut().unwrap();

		vfs.get_file_from_path(&path, uid, gid, true)?
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
