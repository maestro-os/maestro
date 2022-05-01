//! TODO doc

use core::mem::size_of;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::syscall::Regs;
use crate::time::Timespec;
use crate::types::*;

/// The number of file descriptors in FDSet.
const FD_SETSIZE: usize = 1024;

/// Structure representing `fd_set`.
#[repr(C)]
struct FDSet {
	/// The set's bitfield.
	fds_bits: [c_long; FD_SETSIZE / 8 / size_of::<c_long>()],
}

/// The implementation of the `pselect6` syscall.
pub fn pselect6(regs: &Regs) -> Result<i32, Errno> {
	let _nfds = regs.ebx as c_int;
	let _readfds: SyscallPtr<FDSet> = (regs.ecx as usize).into();
	let _writefds: SyscallPtr<FDSet> = (regs.edx as usize).into();
	let _exceptfds: SyscallPtr<FDSet> = (regs.esi as usize).into();
	let _timeout: SyscallPtr<Timespec> = (regs.edi as usize).into();
	let _sigmask: SyscallSlice<u8> = (regs.ebp as usize).into();

	// TODO
	todo!();
}
