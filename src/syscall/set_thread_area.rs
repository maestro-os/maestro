//! This module implements the `set_thread_area` system call, which allows to set a TLS area.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process;

/// Returns the size of the user_desc structure in bytes.
const USER_DESC_SIZE: usize = 13;

/// Returns the ID of a free TLS entry for the given process.
pub fn get_free_entry(process: &mut Process) -> Result<usize, Errno> {
	for (i, e) in process.get_tls_entries().iter().enumerate() {
		if !e.is_present() {
			return Ok(i);
		}
	}

	Err(errno::ESRCH)
}

/// The implementation of the `set_thread_area` syscall.
pub fn set_thread_area(regs: &Regs) -> Result<i32, Errno> {
	let u_info = regs.ebx as *mut [i8; USER_DESC_SIZE];

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	// Checking the process can access the given pointer
	if !proc.get_mem_space().unwrap().can_access(u_info as *const _, USER_DESC_SIZE, true, true) {
		return Err(errno::EFAULT);
	}

	// A reference to the user_desc structure
	let info = unsafe { // Safe because the access was check before
		&mut *u_info
	};

	// TODO Clean
	let entry_number = info[0] as i32 | (info[1] as i32) << 8 | (info[2] as i32) << 16
		| (info[3] as i32) << 24;

	// TODO Move in a separate function
	// Getting the entry for the given number
	let _entry = {
		// Checking the entry number is in bound
		if entry_number < 0 || entry_number > process::TLS_ENTRIES_COUNT as _ {
			return Err(errno::EINVAL);
		}

		// The entry's ID
		let id = {
			if entry_number == -1 {
				// Allocating an entry
				get_free_entry(proc)?
			} else {
				entry_number as usize
			}
		};

		&mut proc.get_tls_entries()[id]
	};

	// TODO Modify the entry

	// If the entry is allocated, tell the userspace its ID
	if entry_number == -1 {
		// TODO Write the entry number in the structure
	}

	Ok(0)
}
