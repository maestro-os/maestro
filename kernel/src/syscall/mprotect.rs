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

//! The `mprotect` system call allows to set permissions for the given range of memory.

use super::{mmap, Args};
use crate::{
	file::perm::AccessProfile,
	memory,
	memory::stats::MemInfo,
	process::{mem_space, mem_space::MemSpace, Process},
	sync::mutex::IntMutex,
};
use core::ffi::{c_int, c_void};
use utils::{
	errno,
	errno::{EResult, Errno},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
};

pub fn mprotect(
	Args((addr, len, prot)): Args<(*mut c_void, usize, c_int)>,
	mem_space: Arc<IntMutex<MemSpace>>,
	ap: AccessProfile,
) -> EResult<usize> {
	// Check alignment of `addr` and `length`
	if !addr.is_aligned_to(PAGE_SIZE) || len == 0 {
		return Err(errno!(EINVAL));
	}
	let prot = prot as u8;
	mem_space.lock().set_prot(addr, len, prot, &ap)?;
	Ok(0)
}
