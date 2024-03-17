/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module implements the `set_thread_area` system call, which allows to
//! set a TLS area.

use crate::{
	gdt, process,
	process::{mem_space::ptr::SyscallPtr, user_desc::UserDesc, Process},
};
use core::mem::size_of;
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// The index of the first entry for TLS segments in the GDT.
const TLS_BEGIN_INDEX: usize = gdt::TLS_OFFSET / size_of::<gdt::Entry>();

/// Returns the ID of a free TLS entry for the given process.
pub fn get_free_entry(process: &mut Process) -> EResult<usize> {
	for (i, e) in process.get_tls_entries().iter().enumerate() {
		if !e.is_present() {
			return Ok(i);
		}
	}

	Err(errno!(ESRCH))
}

/// Returns an entry ID for the given process and entry number.
///
/// If the id is `-1`, the function shall find a free entry.
pub fn get_entry(proc: &mut Process, entry_number: i32) -> EResult<(usize, &mut gdt::Entry)> {
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

#[syscall]
pub fn set_thread_area(u_info: SyscallPtr<UserDesc>) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap().clone();
	let mut mem_space_guard = mem_space.lock();

	// A reference to the user_desc structure
	let info = u_info
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;

	// Get the entry with its id
	let (id, entry) = get_entry(&mut proc, info.get_entry_number())?;

	// Update the entry
	*entry = info.to_descriptor();
	proc.update_tls(id);
	gdt::flush();

	// If the entry is allocated, tell the userspace its ID
	let entry_number = info.get_entry_number();
	if entry_number == -1 {
		info.set_entry_number((TLS_BEGIN_INDEX + id) as _);
	}

	Ok(0)
}
