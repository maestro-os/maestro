//! The `creat` system call allows to create and open a file.

use super::open;
use crate::errno::Errno;
use crate::file;
use crate::file::open_file;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `creat` syscall.
pub fn creat(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let mode = regs.ecx as file::Mode;

	let flags = open_file::O_CREAT | open_file::O_WRONLY | open_file::O_TRUNC;
	open::open_(pathname, flags, mode)
}
