//! The exit_group syscall allows to terminate every processes in the current
//! thread group.

use crate::errno::Errno;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `exit_group` syscall.
#[syscall]
pub fn exit_group(status: c_int) -> Result<i32, Errno> {
	super::_exit::do_exit(status as _, true);
}
