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

//! The `set_thread_area` system call allows to set a TLS area.

use crate::{
	arch::x86::gdt,
	process,
	process::{mem_space::copy::SyscallPtr, user_desc::UserDesc, Process},
	syscall::Args,
};
use core::mem::size_of;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, IntMutexGuard},
	ptr::arc::Arc,
};

/// The index of the first entry for TLS segments in the GDT.
const TLS_BEGIN_INDEX: usize = gdt::TLS_OFFSET / size_of::<gdt::Entry>();

/// Returns an entry ID for the given process and entry number.
///
/// If the id is `-1`, the function shall find a free entry.
fn get_entry(
	entries: &mut [gdt::Entry; process::TLS_ENTRIES_COUNT],
	entry_number: i32,
) -> EResult<(usize, &mut gdt::Entry)> {
	const BEGIN_ENTRY: i32 = TLS_BEGIN_INDEX as i32;
	const END_ENTRY: i32 = BEGIN_ENTRY + process::TLS_ENTRIES_COUNT as i32;
	let id = match entry_number {
		// Find a free entry
		-1 => entries
			.iter()
			.enumerate()
			.find(|(_, e)| !e.is_present())
			.map(|(i, _)| i)
			.ok_or(errno!(ESRCH))?,
		// Valid entry index
		BEGIN_ENTRY..END_ENTRY => (entry_number - BEGIN_ENTRY) as usize,
		// Out of bounds
		_ => return Err(errno!(EINVAL)),
	};
	Ok((id, &mut entries[id]))
}

pub fn set_thread_area(
	Args(u_info): Args<SyscallPtr<UserDesc>>,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Read user_desc
	let mut info = u_info.copy_from_user()?.ok_or(errno!(EFAULT))?;
	// Get the entry with its id
	let mut entries = proc.tls.lock();
	let (id, entry) = get_entry(&mut entries, info.get_entry_number())?;
	// If the entry is allocated, tell the userspace its ID
	let entry_number = info.get_entry_number();
	if entry_number == -1 {
		info.set_entry_number((TLS_BEGIN_INDEX + id) as _);
		u_info.copy_to_user(&info)?;
	}
	// Update the entry
	*entry = info.to_descriptor();
	proc.update_tls(id);
	gdt::flush();
	Ok(0)
}
