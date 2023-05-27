//! The `arch_prctl` system call sets architecture-specific thread state.

use crate::errno::Errno;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn arch_prctl(_code: c_int, _addr: usize) -> Result<i32, Errno> {
	// TODO
	Err(errno!(EINVAL))
}
