//! The exit_group syscall allows to terminate every processes in the current
//! thread group.

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `exit_group` syscall.
pub fn exit_group(regs: &Regs) -> Result<i32, Errno> {
	let status = regs.ebx as i32;

	super::_exit::do_exit(status as _, true);
}
