//! The `mknod` system call allows to create a new node on a filesystem.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `getuid` syscall.
pub fn mknod(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let _mode = regs.ecx as u16;
	let _dev = regs.edx as u16;

	let _path = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		Path::from_str(super::util::get_str(proc, pathname)?, true)?
	};

	// TODO
	Ok(0)
}
