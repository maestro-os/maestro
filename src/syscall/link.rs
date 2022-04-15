//! The link system call allows to create a directory.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `link` syscall.
pub fn link(regs: &Regs) -> Result<i32, Errno> {
	let oldpath: SyscallString = (regs.ebx as usize).into();
	let newpath: SyscallString = (regs.ecx as usize).into();

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let oldpath_str = oldpath.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
	let _old_path = Path::from_str(oldpath_str, true)?;
	let newpath_str = newpath.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
	let _new_path = Path::from_str(newpath_str, true)?;

	// TODO Get file at `old_path`
	// TODO Create the link to the file

	Ok(0)
}
