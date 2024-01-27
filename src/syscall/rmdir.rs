//! The `rmdir` system call a link to the given directory from its filesystem.
//!
//! If no link remain to the directory, the function also removes it.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn rmdir(pathname: SyscallString) -> Result<i32, Errno> {
	let (path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, true);

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = Path::new(path)?;

		(path, rs)
	};

	// Remove the directory
	{
		// Get directory
		let file_mutex = vfs::get_file_from_path(&path, &rs)?;
		let mut file = file_mutex.lock();

		match file.get_content() {
			// The 2 entries in question are `.` and `..`
			FileContent::Directory(entries) if entries.len() > 2 => return Err(errno!(ENOTEMPTY)),
			FileContent::Directory(_) => {}
			_ => return Err(errno!(ENOTDIR)),
		}

		vfs::remove_file_from_path(path, &rs)?;
	}

	Ok(0)
}
