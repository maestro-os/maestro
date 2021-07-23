//! The chdir system call allows to change the current working directory of the current process.

use core::slice;
use core::str;
use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::path::Path;
use crate::file;
use crate::limits;
use crate::process::Process;
use crate::util;

/// The implementation of the `chdir` syscall.
pub fn chdir(regs: &util::Regs) -> Result<i32, Errno> {
	let path = regs.ebx as *const u8;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	// Checking that the buffer is accessible and retrieving the length of the string
	let len = proc.get_mem_space().can_access_string(path, true, false);
	if len.is_none() {
		return Err(errno::EFAULT);
	}
	let len = len.unwrap();

	// Checking the length of the path
	if len > limits::PATH_MAX {
		return Err(errno::ENAMETOOLONG);
	}

	let new_cwd = Path::from_string(unsafe { // Safe because the pointer is checked before
		str::from_utf8_unchecked(slice::from_raw_parts(path, len))
	})?;

	{
		let fcache_mutex = file::get_files_cache();
		let mut fcache_guard = fcache_mutex.lock(true);
		let fcache = fcache_guard.get_mut();

		let mut dir_mutex = fcache.get_file_from_path(&new_cwd)?;
		let dir_guard = dir_mutex.lock(true);
		let dir = dir_guard.get();

		// TODO Check permissions (for all directories in the path)
		if dir.get_file_type() != FileType::Directory {
			return Err(errno::ENOTDIR);
		}
	}

	proc.set_cwd(new_cwd);
	Ok(0)
}
