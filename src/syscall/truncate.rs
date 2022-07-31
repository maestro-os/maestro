//! The truncate syscall allows to truncate a file.

use crate::errno::Errno;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::Process;

/// The implementation of the `truncate` syscall.
pub fn truncate(regs: &Regs) -> Result<i32, Errno> {
	let path: SyscallString = (regs.ebx as usize).into();
	let length = regs.ecx as usize;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let path = Path::from_str(path.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;

	let mutex = fcache::get();
	let guard = mutex.lock();
	let files_cache = guard.get_mut();

	let file_mutex = files_cache.as_mut().unwrap().get_file_from_path(
		&path,
		proc.get_euid(),
		proc.get_egid(),
		true,
	)?;
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();
	file.set_size(length as _);

	Ok(0)
}
