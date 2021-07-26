//! The mkdir system call allows to create a directory.

//use crate::errno;
//use crate::process::Process;
use crate::errno::Errno;
use crate::util;

/// The implementation of the `mkdir` syscall.
pub fn mkdir(regs: &util::Regs) -> Result<i32, Errno> {
	let _pathname = regs.ebx as *const u8;
	let _mode = regs.ebx as u16;

	// TODO
	todo!();
}
