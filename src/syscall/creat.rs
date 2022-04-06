//! The `creat` system call allows to create and open a file.

use crate::errno::Errno;
use crate::file::file_descriptor;
use crate::file;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use super::open;

/// The implementation of the `creat` syscall.
pub fn creat(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let mode = regs.ecx as file::Mode;

	let flags = file_descriptor::O_CREAT | file_descriptor::O_WRONLY | file_descriptor::O_TRUNC;
	open::open_(pathname, flags, mode)
}
