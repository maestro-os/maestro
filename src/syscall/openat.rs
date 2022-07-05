//! The `openat` syscall allows to open a file.

use crate::errno::Errno;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use super::util;

/// The implementation of the `openat` syscall.
pub fn openat(regs: &Regs) -> Result<i32, Errno> {
	let dirfd = regs.ebx as i32;
	let pathname: SyscallString = (regs.ecx as usize).into();
	let _flags = regs.edx as i32;
	let _mode = regs.esi as file::Mode;

	let (file_mutex, _uid, _gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let pathname = pathname.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let file_mutex = util::get_file_at(proc, true, dirfd, pathname, 0)?;
		(file_mutex, proc.get_euid(), proc.get_egid())
	};
	let file_guard = file_mutex.lock();
	let _file = file_guard.get_mut();

	// TODO
	todo!();
}
