//! This `linkat` syscall creates a new hard link to a file.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::types::c_int;
use super::access;

/// The implementation of the `linkat` system call.
pub fn linkat(regs: &Regs) -> Result<i32, Errno> {
	let olddirfd = regs.ebx as c_int;
	let oldpath: SyscallString = (regs.ecx as usize).into();
	let _newdirfd = regs.edx as c_int;
	let _newpath: SyscallString = (regs.esi as usize).into();
	let flags = regs.edi as c_int;

	let follow_links = flags & access::AT_SYMLINK_NOFOLLOW == 0;

	let _old_file_mutex = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let mem_space = proc.get_mem_space().clone().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		super::util::get_file_at(proc_guard, follow_links, olddirfd, oldpath, flags)
	};

	// TODO
	todo!();
}
