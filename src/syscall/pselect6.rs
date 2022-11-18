//! `pselect6` is similar to `select`.

use super::select::do_select;
use super::select::FDSet;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::time::unit::Timespec;
use crate::types::*;
use macros::syscall;

/// The implementation of the `pselect6` syscall.
#[syscall]
pub fn pselect6(nfds: c_int, readfds: SyscallPtr::<FDSet>, writefds: SyscallPtr::<FDSet>, exceptfds: SyscallPtr::<FDSet>, timeout: SyscallPtr::<Timespec>, sigmask: SyscallSlice::<u8>) -> Result<i32, Errno> {
	do_select(
		nfds as _,
		readfds,
		writefds,
		exceptfds,
		timeout,
		Some(sigmask),
	)
}
