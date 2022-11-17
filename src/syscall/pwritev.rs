//! The `pwritev` system call allows to write sparse data on a file descriptor.

use crate::errno::Errno;
use crate::process::iovec::IOVec;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

/// The implementation of the `pwritev` syscall.
pub fn pwritev(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let iov: SyscallSlice<IOVec> = (regs.ecx as usize).into();
	let iovcnt = regs.edx as i32;
	let offset = regs.esi as isize;

	super::writev::do_writev(fd, iov, iovcnt, Some(offset), None)
}
