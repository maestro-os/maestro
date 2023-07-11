//! The `rmdir` system call deletes the given directory from its filesystem.
//!
//! If no link remain to the inode, the function also removes the inode.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn rmdir(pathname: SyscallString) -> Result<i32, Errno> {
	let (path, uid, gid) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		let path = super::util::get_absolute_path(&proc, path)?;

		(path, proc.euid, proc.egid)
	};

	// Removing the directory
	{
		let vfs_mutex = vfs::get();
		let mut vfs = vfs_mutex.lock();
		let vfs = vfs.as_mut().unwrap();

		// Getting directory
		let file_mutex = vfs.get_file_from_path(&path, uid, gid, true)?;
		let file = file_mutex.lock();

		match file.get_content() {
			FileContent::Directory(entries) if entries.len() > 2 => return Err(errno!(ENOTEMPTY)),
			FileContent::Directory(_) => {}

			_ => return Err(errno!(ENOTDIR)),
		}

		vfs.remove_file(&file, uid, gid)?;
	}

	Ok(0)
}
