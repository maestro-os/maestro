//! The `arch_prctl` system call sets architecture-specific thread state.

use core::ffi::c_int;
use crate::errno::Errno;
use macros::syscall;

#[syscall]
pub fn arch_prctl(_code: c_int, _addr: usize) -> Result<i32, Errno> {
	// TODO
	Err(errno!(EINVAL))
}
