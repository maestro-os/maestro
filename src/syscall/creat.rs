//! The `creat` system call allows to create and open a file.

use crate::errno::Errno;
use crate::file::file_descriptor;
use crate::process::Regs;
use super::open;

/// The implementation of the `creat` syscall.
pub fn creat(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let mode = regs.ecx as u16;

	let flags = file_descriptor::O_CREAT | file_descriptor::O_WRONLY | file_descriptor::O_TRUNC;
	open::open_(pathname, flags, mode)
}
