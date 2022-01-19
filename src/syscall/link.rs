//! The link system call allows to create a directory.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `link` syscall.
pub fn link(regs: &Regs) -> Result<i32, Errno> {
	let oldpath = regs.ebx as *const u8;
	let newpath = regs.ecx as *const u8;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let _old_path = Path::from_str(super::util::get_str(proc, oldpath)?, true)?;
	let _new_path = Path::from_str(super::util::get_str(proc, newpath)?, true)?;

	// TODO Get file at `old_path`
	// TODO Create the link to the file

	Ok(0)
}
