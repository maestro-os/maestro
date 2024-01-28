//! The link system call allows to create a directory.

use crate::{
	errno::Errno,
	file::path::Path,
	process::{mem_space::ptr::SyscallString, Process},
};
use macros::syscall;

#[syscall]
pub fn link(oldpath: SyscallString, newpath: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let oldpath_str = oldpath
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let _old_path = Path::new(oldpath_str)?;

	let newpath_str = newpath
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let _new_path = Path::new(newpath_str)?;

	// TODO Get file at `old_path`
	// TODO Create the link to the file

	Ok(0)
}
