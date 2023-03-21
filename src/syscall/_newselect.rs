//! `_newselect` is similar to `select`.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::time::unit::Timeval;
use macros::syscall;
use super::select::FDSet;
use super::select::do_select;

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
