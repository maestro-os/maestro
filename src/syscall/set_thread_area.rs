//! This module implements the `set_thread_area` system call, which allows to set a TLS area.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::gdt;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::process::user_desc::UserDesc;
use crate::process;

/// The index of the first entry for TLS segments in the GDT.
const TLS_BEGIN_INDEX: usize = gdt::TLS_OFFSET / size_of::<gdt::Entry>();

/// Returns the ID of a free TLS entry for the given process.
pub fn get_free_entry(process: &mut Process) -> Result<usize, Errno> {
	for (i, e) in process.get_tls_entries().iter().enumerate() {
		if !e.is_present() {
			return Ok(i);
		}
	}

	Err(errno!(ESRCH))
}

/// Returns an entry ID for the given process and entry number.
/// If the id is `-1`, the function shall find a free entry.
pub fn get_entry<'a>(proc: &'a mut Process, entry_number: i32)
	-> Result<(usize, &'a mut gdt::Entry), Errno> {
	let end_entry = (TLS_BEGIN_INDEX + process::TLS_ENTRIES_COUNT) as i32;

	// Checking the entry number is in bound
	if entry_number != -1 && entry_number < TLS_BEGIN_INDEX as i32 || entry_number > end_entry {
		return Err(errno!(EINVAL));
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
	let u_info: SyscallPtr<UserDesc> = (regs.ebx as usize).into();

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// A reference to the user_desc structure
	let info = u_info.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
	crate::println!("user desc: {}", info); // TODO rm

	// Getting the entry with its id
	let (id, entry) = get_entry(proc, info.get_entry_number())?;

	// Updating the entry
	*entry = info.to_descriptor();
	// Updating the GDT
	proc.update_tls(id);

	// If the entry is allocated, tell the userspace its ID
	let entry_number = info.get_entry_number();
	if entry_number == -1 {
		info.set_entry_number((TLS_BEGIN_INDEX + id) as _);
	}

	Ok(0)
}
