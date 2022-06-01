/// This module implements utility functions for system calls.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// Returns the absolute path according to the process's current working directory.
/// `process` is the process.
/// `path` is the path.
pub fn get_absolute_path(process: &Process, path: Path) -> Result<Path, Errno> {
	if !path.is_absolute() {
		let cwd = process.get_cwd();
		cwd.concat(&path)
	} else {
		Ok(path)
	}
}

// TODO Find a safer and cleaner solution
/// Checks that the given array of strings at pointer `ptr` is accessible to process `proc`, then
/// returns its content.
/// If the array or its content strings are not accessible by the process, the function returns an
/// error.
pub unsafe fn get_str_array(process: &Process, ptr: *const *const u8)
	-> Result<Vec<String>, Errno> {
	let mem_space = process.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Checking every elements of the array and counting the number of elements
	let mut len = 0;
	loop {
		let elem_ptr = ptr.add(len);

		// Checking access on elem_ptr
		if !mem_space_guard.get().can_access(elem_ptr as _, size_of::<*const u8>(), true, false) {
			return Err(errno!(EFAULT));
		}

		// Safe because the access is checked before
		let elem = *elem_ptr;
		if elem.is_null() {
			break;
		}

		len += 1;
	}

	// Filling the array
	let mut arr = Vec::with_capacity(len)?;
	for i in 0..len {
		let elem = *ptr.add(i);
		let s: SyscallString = (elem as usize).into();

		arr.push(String::from(s.get(&mem_space_guard)?.unwrap())?)?;
	}

	Ok(arr)
}
