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

	let new_cwd = Path::from_str(super::util::get_str(proc, path)?, true)?;

	{
		let fcache_mutex = fcache::get();
		let mut fcache_guard = fcache_mutex.lock();
		let fcache = fcache_guard.get_mut();

		let dir_mutex = fcache.as_mut().unwrap().get_file_from_path(&new_cwd)?;
		let dir_guard = dir_mutex.lock();
		let dir = dir_guard.get();

		// TODO Check permissions (for all directories in the path)
		if dir.get_file_type() != FileType::Directory {
			return Err(errno::ENOTDIR);
		}
	}

	// TODO Make the path absolute
	proc.set_cwd(new_cwd);
	Ok(0)
}
