//! The `statfs` system call returns information about a mounted file system.

use crate::errno;
use crate::errno::{EResult, Errno};
use crate::file::fs::Statfs;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

pub(super) fn do_statfs(path: SyscallString, buf: SyscallPtr<Statfs>) -> EResult<i32> {
	let (path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = path.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let path = Path::new(path)?;

		let rs = ResolutionSettings::for_process(&proc, false);
		(path, rs)
	};

	let stat = {
		let file_mutex = vfs::get_file_from_path(&path, &rs)?;
		let file = file_mutex.lock();

		// Unwrapping will not fail since the file is accessed from path
		let mountpoint_mutex = file.get_location().get_mountpoint().unwrap();
		let mountpoint = mountpoint_mutex.lock();

		let io_mutex = mountpoint.get_source().get_io()?;
		let mut io = io_mutex.lock();

		let fs_mutex = mountpoint.get_filesystem();
		let fs = fs_mutex.lock();

		fs.get_stat(&mut *io)?
	};

	// Write structure to userspace
	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mut mem_space_guard = mem_space.lock();

		let buf = buf
			.get_mut(&mut mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		*buf = stat;
	}

	Ok(0)
}

#[syscall]
pub fn statfs(path: SyscallString, buf: SyscallPtr<Statfs>) -> Result<i32, Errno> {
	do_statfs(path, buf)
}
