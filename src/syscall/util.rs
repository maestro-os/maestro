/// This module implements utility functions for system calls.

use core::mem::size_of;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::path::Path;
use crate::process::Process;
use crate::util::container::vec::Vec;

/// Checks that the string at the given pointer `s` is accessible by the process `proc`, then
/// returns a slice to it.
/// If the string is not accessible by the process, the function returns an error.
pub fn get_str(proc: &Process, s: *const u8) -> Result<&'static [u8], Errno> {
	let mem_space = proc.get_mem_space().unwrap();

	// Getting the length and checking access
	let len = mem_space.can_access_string(s as _, true, false).ok_or(errno!(EFAULT))?;
	// Getting the slice
	let slice = unsafe { // Safe because the access is checked before
		slice::from_raw_parts(s, len)
	};

	Ok(slice)
}

/// Returns the absolute path according to the process's current working directory.
/// `process` is the process.
/// `path` is the path.
pub fn get_absolute_path(process: &Process, path: Path) -> Result<Path, Errno> {
	if !path.is_absolute() {
		let cwd = process.get_cwd();
		let mut absolute_path = cwd.concat(&path)?;
		absolute_path.reduce()?;

		Ok(absolute_path)
	} else {
		Ok(path)
	}
}

/// Checks that the given array of strings at pointer `ptr` is accessible to process `proc`, then
/// returns its content.
/// If the array or its content strings are not accessible by the process, the function returns an
/// error.
pub fn get_str_array(process: &Process, ptr: *const *const u8)
	-> Result<Vec<&'static [u8]>, Errno> {
	let mem_space = process.get_mem_space().unwrap();

	// Checking every elements of the array and counting the number of elements
	let mut len = 0;
	loop {
		let elem_ptr = unsafe {
			ptr.add(len)
		};

		// Checking access on elem_ptr
		if !mem_space.can_access(elem_ptr as _, size_of::<*const u8>(), true, false) {
			return Err(errno!(EFAULT));
		}

		// Safe because the access is checked before
		let elem = unsafe { *elem_ptr };

		if elem.is_null() {
			break;
		}

		len += 1;
	}

	// Filling the array
	let mut arr = Vec::with_capacity(len)?;
	for i in 0..len {
		let elem = unsafe {
			*ptr.add(i)
		};
		let s = get_str(process, elem)?;

		arr.push(s)?;
	}

	Ok(arr)
}
