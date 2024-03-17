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

//! The `munmap` system call allows the process to free memory that was
//! allocated with `mmap`.

use crate::{memory, process::Process};
use core::{ffi::c_void, num::NonZeroUsize};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn munmap(addr: *mut c_void, length: usize) -> Result<i32, Errno> {
	if !addr.is_aligned_to(memory::PAGE_SIZE) || length == 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let pages = length.div_ceil(memory::PAGE_SIZE);
	let length = pages * memory::PAGE_SIZE;

	// Checking for overflow
	let end = (addr as usize).wrapping_add(length);
	if end < addr as usize {
		return Err(errno!(EINVAL));
	}

	// Prevent from unmapping kernel memory
	if (addr as usize) >= (memory::PROCESS_END as usize) || end > (memory::PROCESS_END as usize) {
		return Err(errno!(EINVAL));
	}

	proc.get_mem_space()
		.unwrap()
		.lock()
		.unmap(addr, NonZeroUsize::new(pages).unwrap(), false)?;
	Ok(0)
}
