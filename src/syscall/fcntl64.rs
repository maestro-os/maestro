//! The `fcntl64` syscall call allows to manipulate a file descriptor.

use crate::errno::Errno;
use crate::process::regs::Regs;
use core::ffi::c_void;

/// The implementation of the `fcntl64` syscall.
pub fn fcntl64(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let cmd = regs.ecx as i32;
	let arg = regs.edx as *mut c_void;

	super::fcntl::do_fcntl(fd, cmd, arg, true)
}
