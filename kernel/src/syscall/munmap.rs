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

use crate::{
	memory,
	process::{mem_space::MemSpace, Process},
	syscall::Args,
};
use core::{ffi::c_void, num::NonZeroUsize};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::IntMutex,
	ptr::arc::Arc,
};

pub fn munmap(
	Args((addr, length)): Args<(*mut c_void, usize)>,
	mem_space: Arc<IntMutex<MemSpace>>,
) -> EResult<usize> {
	// Check address alignment
	if !addr.is_aligned_to(memory::PAGE_SIZE) || length == 0 {
		return Err(errno!(EINVAL));
	}
	let pages = length.div_ceil(memory::PAGE_SIZE);
	let length = pages * memory::PAGE_SIZE;
	// Check for overflow
	let Some(end) = (addr as usize).checked_add(length) else {
		return Err(errno!(EINVAL));
	};
	// Prevent from unmapping kernel memory
	if (addr as usize) >= (memory::PROCESS_END as usize) || end > (memory::PROCESS_END as usize) {
		return Err(errno!(EINVAL));
	}
	mem_space
		.lock()
		.unmap(addr, NonZeroUsize::new(pages).unwrap(), false)?;
	Ok(0)
}
