//! The `chroot` system call allows to virtually redefine the system's root for
//! the current process.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs::ResolutionSettings;
use crate::file::{FileLocation, FileType};
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::vfs;
use macros::syscall;

#[syscall]
pub fn chroot(path: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();
	// Check permission
	if !proc.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}

	let rs = ResolutionSettings::for_process(&proc, true);
	rs.root = FileLocation::root();

	// Get file
	let file = {
		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = path.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = Path::new(path)?;

		vfs::get_file_from_path(&path, &rs)?
	};
	let file = file.lock();
	if file.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}

	proc.chroot = file.get_location().clone();

	Ok(0)
}
