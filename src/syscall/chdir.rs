//! The chdir system call allows to change the current working directory of the current process.

use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `chdir` syscall.
pub fn chdir(regs: &Regs) -> Result<i32, Errno> {
	let path = regs.ebx as *const u8;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let path_str = super::util::get_str(proc, path)?;
	let new_cwd = super::util::get_absolute_path(&proc, Path::from_str(path_str, true)?)?;

	{
		let fcache_mutex = fcache::get();
		let mut fcache_guard = fcache_mutex.lock();
		let fcache = fcache_guard.get_mut();

		let dir_mutex = fcache.as_mut().unwrap().get_file_from_path(&new_cwd, proc.get_euid(),
			proc.get_egid())?;
		let dir_guard = dir_mutex.lock();
		let dir = dir_guard.get();

		// Checking for errors
		if !dir.can_read(proc.get_euid(), proc.get_egid()) {
			return Err(errno::EACCES);
		}
		if dir.get_file_type() != FileType::Directory {
			return Err(errno::ENOTDIR);
		}
	}

	proc.set_cwd(new_cwd)?;
	Ok(0)
}
