//! The `unlink` system call deletes the given file from its filesystem. If no link remain to the
//! inode, the function also removes the inode.

use crate::errno::Errno;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `unlink` syscall.
pub fn unlink(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;

	let path = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		Path::from_str(super::util::get_str(proc, pathname)?, true)?
	};

	// TODO If the file is busy, remove only when the last fd to it is closed

	// Removing the file
	{
		let mutex = fcache::get();
		let mut guard = mutex.lock();
		let files_cache = guard.get_mut();

		files_cache.as_mut().unwrap().remove_file(&path)?;
	}

	Ok(0)
}
