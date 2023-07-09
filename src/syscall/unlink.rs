//! The `unlink` system call deletes the given file from its filesystem.
//!
//! If no link remain to the inode, the function also removes the inode.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn unlink(pathname: SyscallString) -> Result<i32, Errno> {
	let (path, uid, gid) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();
		let path = Path::from_str(pathname.get(&mem_space)?.ok_or(errno!(EFAULT))?, true)?;
		let path = super::util::get_absolute_path(&proc, path)?;

		(path, proc.euid, proc.egid)
	};

	// Removing the file
	{
		let vfs_mutex = vfs::get();
		let mut vfs = vfs_mutex.lock();
		let vfs = vfs.as_mut().unwrap();

		// Getting file
		let file_mutex = vfs.get_file_from_path(&path, uid, gid, true)?;
		let file = file_mutex.lock();

		vfs.remove_file(&file, uid, gid)?;
	}

	Ok(0)
}
