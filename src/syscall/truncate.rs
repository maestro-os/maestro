//! The truncate syscall allows to truncate a file.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `truncate` syscall.
#[syscall]
pub fn truncate(path: SyscallString, length: usize) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let path = Path::from_str(path.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
	let path = super::util::get_absolute_path(proc, path)?;

	let mutex = vfs::get();
	let guard = mutex.lock();
	let vfs = guard.get_mut();

	let file_mutex =
		vfs.as_mut()
			.unwrap()
			.get_file_from_path(&path, proc.get_euid(), proc.get_egid(), true)?;
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();
	file.set_size(length as _);

	Ok(0)
}
