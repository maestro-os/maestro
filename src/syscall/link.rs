//! The link system call allows to create a directory.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `link` syscall.
#[syscall]
pub fn link(oldpath: SyscallString, newpath: SyscallString) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let oldpath_str = oldpath
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let old_path = Path::from_str(oldpath_str, true)?;
	let _old_path = super::util::get_absolute_path(proc, old_path)?;

	let newpath_str = newpath
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let new_path = Path::from_str(newpath_str, true)?;
	let _new_path = super::util::get_absolute_path(proc, new_path)?;

	// TODO Get file at `old_path`
	// TODO Create the link to the file

	Ok(0)
}
