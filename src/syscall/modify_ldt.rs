//! This module implements the `set_thread_area` system call, which allows to set a LDT entry for
//! the process.

use crate::errno::Errno;
use crate::process::Regs;

/// The implementation of the `set_thread_area` syscall.
pub fn modify_ldt(_regs: &Regs) -> Result<i32, Errno> {
	// TODO

	Ok(0)
}
