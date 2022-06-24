//! The `unlinkat` syscall allows to unlink a file.

use crate::errno::Errno;
use crate::file::fcache;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use super::util;

/// The implementation of the `unlinkat` syscall.
pub fn unlinkat(regs: &Regs) -> Result<i32, Errno> {
	let dirfd = regs.ebx as i32;
	let pathname: SyscallString = (regs.ecx as usize).into();
	let flags = regs.edx as i32;

	// Getting the process
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let file_mutex = util::get_file_at(proc, false, dirfd, pathname, flags)?;
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();

	let mutex = fcache::get();
	let guard = mutex.lock();
	let files_cache = guard.get_mut().as_mut().unwrap();
	files_cache.remove_file(file, proc.get_euid(), proc.get_egid())?;

	Ok(0)
}
