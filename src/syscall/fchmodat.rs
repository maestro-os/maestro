//! The `fchmodat` system call allows change the permissions on a file.

use crate::errno::Errno;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use super::util;

/// The implementation of the `fchmodat` syscall.
pub fn fchmodat(regs: &Regs) -> Result<i32, Errno> {
	let dirfd = regs.ebx as i32;
	let pathname: SyscallString = (regs.ecx as usize).into();
	let mode = regs.ecx as file::Mode;
	let _flags = regs.edx as i32;

	let (file_mutex, uid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let pathname = pathname.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let file_mutex = util::get_file_at(proc, true, dirfd, pathname, 0)?;
		(file_mutex, proc.get_euid())
	};
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();

	// Checking permissions
	if uid != file::ROOT_UID && uid != file.get_uid() {
		return Err(errno!(EPERM));
	}

	file.set_permissions(mode);

	Ok(0)
}
