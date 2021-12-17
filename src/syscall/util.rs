/// This module implements utility functions for system calls.

use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;

/// Checks that the string at the given pointer `s` is accessible by the process `proc`, then
/// returns a slice to it.
/// If the string is not accessible by the process, the function returns an error.
pub fn get_str(proc: &Process, s: *const u8) -> Result<&'static [u8], Errno> {
	let mem_space = proc.get_mem_space().unwrap();

	// Getting the length and checking access
	let len = mem_space.can_access_string(s as _, true, false).ok_or(errno::EFAULT)?;
	// Getting the slice
	let slice = unsafe { // Safe because the access is checked before
		slice::from_raw_parts(s, len)
	};

	Ok(slice)
}
