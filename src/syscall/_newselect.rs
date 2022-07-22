//! `_newselect` is similar to `select`.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::syscall::Regs;
use crate::time::unit::Timeval;
use crate::types::*;
use super::select::FDSet;
use super::select::do_select;

/// The implementation of the `_newselect` system call.
pub fn _newselect(regs: &Regs) -> Result<i32, Errno> {
	let nfds = regs.ebx as c_int;
	let readfds: SyscallPtr<FDSet> = (regs.ecx as usize).into();
	let writefds: SyscallPtr<FDSet> = (regs.edx as usize).into();
	let exceptfds: SyscallPtr<FDSet> = (regs.esi as usize).into();
	let timeout: SyscallPtr<Timeval> = (regs.edi as usize).into();

	do_select(nfds as _, readfds, writefds, exceptfds, timeout, None)
}
