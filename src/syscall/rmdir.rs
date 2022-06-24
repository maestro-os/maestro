//! The `rmdir` system call deletes the given directory from its filesystem. If no link remain to
//! the inode, the function also removes the inode.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `rmdir` syscall.
pub fn rmdir(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();

	let (path, uid, gid) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		(path, proc.get_euid(), proc.get_egid())
	};

	// Removing the directory
	{
		let mutex = fcache::get();
		let guard = mutex.lock();
		let files_cache = guard.get_mut().as_mut().unwrap();

		// Getting directory
		let file_mutex = files_cache.get_file_from_path(&path, uid, gid, true)?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		match file.get_file_content() {
			FileContent::Directory(entries) if !entries.is_empty() => return Err(errno!(ENOTDIR)),
			FileContent::Directory(_) => {},

			_ => return Err(errno!(ENOTDIR)),
		}

		files_cache.remove_file(file, uid, gid)?;
	}

	Ok(0)
}
