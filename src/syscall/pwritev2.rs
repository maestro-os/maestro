//! The `pwritev2` system call allows to write sparse data on a file descriptor.

use crate::errno::Errno;
use crate::process::iovec::IOVec;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

/// The implementation of the `pwritev2` syscall.
pub fn pwritev2(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let iov: SyscallSlice<IOVec> = (regs.ecx as usize).into();
	let iovcnt = regs.edx as i32;
	let offset = regs.esi as isize;
	let flags = regs.edi as i32;

	super::writev::do_writev(fd, iov, iovcnt, Some(offset), Some(flags))
}
