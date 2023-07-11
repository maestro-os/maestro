//! The `unlinkat` syscall allows to unlink a file.

use super::util;
use crate::errno::Errno;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn unlinkat(dirfd: c_int, pathname: SyscallString, flags: c_int) -> Result<i32, Errno> {
	let (file_mutex, uid, gid) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let uid = proc.euid;
		let gid = proc.egid;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;

		let file = util::get_file_at(proc, false, dirfd, pathname, flags)?;

		(file, uid, gid)
	};
	let file = file_mutex.lock();

	let vfs_mutex = vfs::get();
	let mut vfs = vfs_mutex.lock();
	let vfs = vfs.as_mut().unwrap();

	vfs.remove_file(&file, uid, gid)?;

	Ok(0)
}
