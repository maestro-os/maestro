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
	let (path, mode, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mode = mode & !proc.umask;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		// Path to the directory to create
		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = Path::from_str(path, true)?;
		let path = super::util::get_absolute_path(&proc, path)?;

		(path, mode, proc.access_profile)
	};

	// Get path of the parent directory and name of the directory to create
	let mut parent_path = path;
	let name = parent_path.pop();

	// If the path is not empty, create
	if let Some(name) = name {
		// Get parent directory
		let parent_mutex = vfs::get_file_from_path(&parent_path, &ap, true)?;
		let mut parent = parent_mutex.lock();

		// Create the directory
		vfs::create_file(
			&mut parent,
			name,
			ap,
			mode,
			FileContent::Directory(HashMap::new()),
		)?;
	}

	Ok(0)
}
