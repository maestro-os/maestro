//! This module implements the `set_thread_area` system call, which allows to set a TLS area.

use crate::errno::Errno;
use crate::errno;
use crate::gdt;
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

/// Returns an entry ID for the given process and entry number.
/// If the id is `-1`, the function shall find a free entry.
pub fn get_entry<'a>(proc: &'a mut Process, entry_number: i32)
    -> Result<(usize, &'a mut gdt::Entry), Errno> {
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

	Ok((id, &mut proc.get_tls_entries()[id]))
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

	// The entry number
	let entry_number = unsafe { // Safe because the structure is large enough
        &mut *(&mut info[0] as *mut _ as *mut i32)
	};

	// Getting the entry with its id
	let (id, entry) = get_entry(proc, *entry_number as _)?;
	debug_assert!(id < process::TLS_ENTRIES_COUNT);

	let base_addr = unsafe { // Safe because the structure is large enough
        &*(&mut info[4] as *const _ as *const i32)
	};
	let limit = unsafe { // Safe because the structure is large enough
        &*(&mut info[8] as *const _ as *const i32)
	};
	let _flags = unsafe { // Safe because the structure is large enough
        &*(&mut info[12] as *const _ as *const i32)
	};

	entry.set_base(*base_addr as _);
	entry.set_limit(*limit as _);
	// TODO Modify the of other fields of the entry
	entry.set_present(true); // TODO Handle clearing

    // Updating the GDT
	proc.update_tls(id);

	// If the entry is allocated, tell the userspace its ID
	if (*entry_number as i32) == -1 {
	    *entry_number = id as _;
	}

	Ok(0)
}
