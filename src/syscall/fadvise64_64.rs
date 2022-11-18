//! The `fadvise64_64` syscall gives hints to the kernel about file accesses.

use core::ffi::c_int;
use crate::errno::Errno;
use macros::syscall;

// TODO Check args type
/// The implementation of the `setgid32` syscall.
#[syscall]
pub fn fadvise64_64(_fd: c_int, _offset: u64, _len: u64, _advice: c_int) -> Result<i32, Errno> {
	// TODO
	Ok(0)
}
