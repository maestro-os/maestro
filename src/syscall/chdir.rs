//! The chdir system call allows to change the current working directory of the current process.

use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `chdir` syscall.
pub fn chdir(regs: &Regs) -> Result<i32, Errno> {
	let path: SyscallString = (regs.ebx as usize).into();

	let (new_cwd, uid, gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let uid = proc.get_euid();
		let gid = proc.get_egid();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path_str = path.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;

		let new_cwd = super::util::get_absolute_path(proc, Path::from_str(path_str, true)?)?;
		(new_cwd, uid, gid)
	};

	{
		let fcache_mutex = fcache::get();
		let fcache_guard = fcache_mutex.lock();
		let fcache = fcache_guard.get_mut();

		let dir_mutex = fcache.as_mut().unwrap().get_file_from_path(&new_cwd, uid, gid, true)?;
		let dir_guard = dir_mutex.lock();
		let dir = dir_guard.get();

		// Checking for errors
		if !dir.can_read(uid, gid) {
			return Err(errno!(EACCES));
		}
		if dir.get_file_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
	}

	// Setting new cwd
	{
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();
		proc.set_cwd(new_cwd)?;
	}

	Ok(0)
}
