//! The chdir system call allows to change the current working directory of the
//! current process.

use crate::errno;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::ptr::arc::Arc;
use macros::syscall;

#[syscall]
pub fn chdir(path: SyscallString) -> Result<i32, Errno> {
	let (new_cwd, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = path.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let path = Path::new(path)?;
		let new_cwd = super::util::get_absolute_path(&proc, path)?;

		(new_cwd, proc.access_profile)
	};

	{
		let dir_mutex = vfs::get_file_from_path(&new_cwd, &ap, true)?;
		let dir = dir_mutex.lock();

		// Check for errors
		if dir.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !ap.can_list_directory(&dir) {
			return Err(errno!(EACCES));
		}
	}

	// Set new cwd
	{
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();
		proc.cwd = Arc::new(new_cwd)?;
	}

	Ok(0)
}
