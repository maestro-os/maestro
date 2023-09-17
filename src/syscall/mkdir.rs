//! The mkdir system call allows to create a directory.

use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::container::hashmap::HashMap;
use macros::syscall;

#[syscall]
pub fn mkdir(pathname: SyscallString, mode: file::Mode) -> Result<i32, Errno> {
	let (path, mode, uid, gid) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mode = mode & !proc.umask;
		let uid = proc.uid;
		let gid = proc.gid;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		// Path to the directory to create
		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let mut path = Path::from_str(path, true)?;
		path = super::util::get_absolute_path(&proc, path)?;

		(path, mode, uid, gid)
	};

	// Get path of the parent directory
	let mut parent_path = path;

	// If the path is not empty, create
	if let Some(name) = parent_path.pop() {
		// Creating the directory
		{
			// Get parent directory
			let parent_mutex = vfs::get_file_from_path(&parent_path, uid, gid, true)?;
			let mut parent = parent_mutex.lock();

			vfs::create_file(
				&mut parent,
				name,
				uid,
				gid,
				mode,
				FileContent::Directory(HashMap::new()),
			)?;
		}
	}

	Ok(0)
}
