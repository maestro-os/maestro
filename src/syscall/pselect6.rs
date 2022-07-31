//! `pselect6` is similar to `select`.

use super::select::do_select;
use super::select::FDSet;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::syscall::Regs;
use crate::time::unit::Timespec;
use crate::types::*;

/// The implementation of the `pselect6` syscall.
pub fn pselect6(regs: &Regs) -> Result<i32, Errno> {
	let nfds = regs.ebx as c_int;
	let readfds: SyscallPtr<FDSet> = (regs.ecx as usize).into();
	let writefds: SyscallPtr<FDSet> = (regs.edx as usize).into();
	let exceptfds: SyscallPtr<FDSet> = (regs.esi as usize).into();
	let timeout: SyscallPtr<Timespec> = (regs.edi as usize).into();
	let sigmask: SyscallSlice<u8> = (regs.ebp as usize).into();

	do_select(
		nfds as _,
		readfds,
		writefds,
		exceptfds,
		timeout,
		Some(sigmask),
	)
}
