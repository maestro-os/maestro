//! `pselect6` is similar to `select`.

use super::select::{do_select, FDSet};
use crate::{
	errno::Errno,
	process::mem_space::ptr::{SyscallPtr, SyscallSlice},
	time::unit::Timespec,
};
use core::ffi::c_int;
use macros::syscall;

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
