//! TODO doc

use core::ffi::c_int;
use crate::errno::Errno;
use macros::syscall;

#[syscall]
pub fn arch_prctl(_code: c_int, _addr: usize) -> Result<i32, Errno> {
	// TODO
	Ok(0)
}
