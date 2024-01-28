//! The mkdir system call allows to create a directory.

use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::file::path::PathBuf;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::container::hashmap::HashMap;
use macros::syscall;

#[syscall]
pub fn mkdir(pathname: SyscallString, mode: file::Mode) -> Result<i32, Errno> {
	let (path, mode, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mode = mode & !proc.umask;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		// Path to the directory to create
		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let rs = ResolutionSettings::for_process(&proc, true);
		(path, mode, rs)
	};

	// If the path is not empty, create
	if let Some(name) = path.file_name() {
		// Get parent directory
		let parent_path = path.parent().unwrap_or(Path::root());
		let parent_mutex = vfs::get_file_from_path(parent_path, &rs)?;
		let mut parent = parent_mutex.lock();

		// Create the directory
		vfs::create_file(
			&mut parent,
			name.try_into()?,
			&rs.access_profile,
			mode,
			FileContent::Directory(HashMap::new()),
		)?;
	}

	Ok(0)
}
