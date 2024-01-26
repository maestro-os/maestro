//! The chdir system call allows to change the current working directory of the
//! current process.

use crate::errno;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::ptr::arc::Arc;
use macros::syscall;

#[syscall]
pub fn chdir(path: SyscallString) -> Result<i32, Errno> {
	let (path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = path.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let path = Path::new(path)?;
		let path = super::util::get_absolute_path(&proc, path)?;

		let rs = ResolutionSettings::for_process(&proc, true);
		(path, rs)
	};

	let location = {
		let dir_mutex = vfs::get_file_from_path(&path, &rs)?;
		let dir = dir_mutex.lock();

		// Check for errors
		if dir.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !rs.access_profile.can_list_directory(&dir) {
			return Err(errno!(EACCES));
		}

		dir.get_location().clone()
	};

	// Set new cwd
	{
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();
		proc.cwd = Arc::new((path, location))?;
	}

	Ok(0)
}
