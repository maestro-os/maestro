//! `_newselect` is similar to `select`.

use super::select::{do_select, FDSet};
use crate::{errno::Errno, process::mem_space::ptr::SyscallPtr, time::unit::Timeval};
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn _newselect(
	nfds: c_int,
	readfds: SyscallPtr<FDSet>,
	writefds: SyscallPtr<FDSet>,
	exceptfds: SyscallPtr<FDSet>,
	timeout: SyscallPtr<Timeval>,
) -> Result<i32, Errno> {
	do_select(nfds as _, readfds, writefds, exceptfds, timeout, None)
}
