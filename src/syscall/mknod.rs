//! The `mknod` system call allows to create a new node on a filesystem.

use crate::errno::Errno;
use crate::errno;
use crate::limits;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `getuid` syscall.
pub fn mknod(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let _mode = regs.ecx as u16;
	let _dev = regs.edx as u16;

	// The length of the pathname
	let _len = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		// Check the pathname is accessible by the process
		let len = proc.get_mem_space().unwrap().can_access_string(pathname as _, true, false);
		if len.is_none() {
			return Err(errno::EFAULT);
		}
		let len = len.unwrap();
		if len > limits::PATH_MAX {
			return Err(errno::ENAMETOOLONG);
		}

		len
	};

	// TODO
	Ok(0)
}
