//! The `connect` system call connects a socket to a distant host.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

/// The implementation of the `connect` syscall.
pub fn connect(regs: &Regs) -> Result<i32, Errno> {
	let _sockfd = regs.ebx as i32;
	let _addr: SyscallSlice<u8> = (regs.ecx as usize).into();
	let _addrlen = regs.edx as usize;

	// TODO
	todo!();
}
