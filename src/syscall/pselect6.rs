//! `pselect6` is similar to `select`.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::time::unit::Timespec;
use macros::syscall;
use super::select::FDSet;
use super::select::do_select;

#[syscall]
pub fn pselect6(
	nfds: c_int,
	readfds: SyscallPtr<FDSet>,
	writefds: SyscallPtr<FDSet>,
	exceptfds: SyscallPtr<FDSet>,
	timeout: SyscallPtr<Timespec>,
	sigmask: SyscallSlice<u8>,
) -> Result<i32, Errno> {
	do_select(
		nfds as _,
		readfds,
		writefds,
		exceptfds,
		timeout,
		Some(sigmask),
	)
}
