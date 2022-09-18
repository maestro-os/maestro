//! The `unlink` system call deletes the given file from its filesystem. If no link remain to the
//! inode, the function also removes the inode.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `unlink` syscall.
pub fn unlink(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();

	let (path, uid, gid) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		let path = super::util::get_absolute_path(proc, path)?;

		(path, proc.get_euid(), proc.get_egid())
	};

	// Removing the file
	{
		let mutex = vfs::get();
		let guard = mutex.lock();
		let vfs = guard.get_mut().as_mut().unwrap();

		// Getting file
		let file_mutex = vfs.get_file_from_path(&path, uid, gid, true)?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		vfs.remove_file(file, uid, gid)?;
	}

	Ok(0)
}
