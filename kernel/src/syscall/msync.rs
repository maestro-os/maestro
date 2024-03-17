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

//! The msync system call synchronizes a memory mapping with its file on the
//! disk.

use crate::{memory, process::Process};
use core::ffi::{c_int, c_void};
use macros::syscall;
use utils::{errno, errno::Errno};

/// Schedules a synchronization and returns directly.
const MS_ASYNC: i32 = 0b001;
/// Synchronizes the mapping before returning.
const MS_SYNC: i32 = 0b010;
/// Invalides other mappings of the same file so they can be updated.
const MS_INVALIDATE: i32 = 0b100;

#[syscall]
pub fn msync(addr: *mut c_void, length: usize, flags: c_int) -> Result<i32, Errno> {
	// Check address alignment
	if !addr.is_aligned_to(memory::PAGE_SIZE) {
		return Err(errno!(EINVAL));
	}
	// Check for conflicts in flags
	if flags & MS_ASYNC != 0 && flags & MS_SYNC != 0 {
		return Err(errno!(EINVAL));
	}
	let pages = length.div_ceil(memory::PAGE_SIZE);

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// The process's memory space
	let mem_space = proc.get_mem_space().unwrap();
	let mem_space = mem_space.lock();

	let mut i = 0;
	while i < pages {
		let mapping = mem_space.get_mapping_for_ptr(addr).ok_or(errno!(ENOMEM))?;
		mapping.fs_sync(mem_space.get_vmem())?; // TODO Use flags
		i += mapping.get_size().get();
	}

	Ok(0)
}
