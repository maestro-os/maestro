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

//! Memory management system calls.

use crate::{
	file::{FileType, fd::fd_to_file},
	memory,
	memory::{VirtAddr, user::UserSlice},
	process::{
		Process,
		mem_space::{MAP_ANONYMOUS, MAP_SHARED, PROT_WRITE},
	},
};
use core::{
	ffi::{c_int, c_void},
	hint::unlikely,
	num::NonZeroUsize,
};
use utils::{errno, errno::EResult, limits::PAGE_SIZE};

/// Performs the `mmap` system call.
#[allow(clippy::too_many_arguments)]
pub fn do_mmap(
	addr: VirtAddr,
	length: usize,
	prot: i32,
	flags: i32,
	fd: i32,
	offset: u64,
) -> EResult<usize> {
	// The length in number of pages
	let pages = length.div_ceil(PAGE_SIZE);
	let Some(pages) = NonZeroUsize::new(pages) else {
		return Err(errno!(EINVAL));
	};
	let prot = prot as u8;
	let file = if flags & MAP_ANONYMOUS == 0 {
		// Validation
		if unlikely(fd < 0) {
			return Err(errno!(EBADF));
		}
		if unlikely(offset as usize % PAGE_SIZE != 0) {
			return Err(errno!(EINVAL));
		}
		// Get file
		let file = fd_to_file(fd)?;
		// Check permissions
		if unlikely(file.stat().get_type() != Some(FileType::Regular)) {
			return Err(errno!(EACCES));
		}
		if unlikely(flags & MAP_SHARED != 0 && prot & PROT_WRITE != 0 && !file.can_write()) {
			return Err(errno!(EACCES));
		}
		Some(file)
	} else {
		None
	};
	let addr = Process::current()
		.mem_space()
		.map(addr, pages, prot, flags, file, offset)?;
	Ok(addr.0 as _)
}

pub fn mmap(
	addr: VirtAddr,
	length: usize,
	prot: c_int,
	flags: c_int,
	fd: c_int,
	offset: u64,
) -> EResult<usize> {
	do_mmap(addr, length, prot, flags, fd, offset as _)
}

pub fn mmap2(
	addr: VirtAddr,
	length: usize,
	prot: c_int,
	flags: c_int,
	fd: c_int,
	offset: u64,
) -> EResult<usize> {
	do_mmap(addr, length, prot, flags, fd, offset * 4096)
}

pub fn brk(addr: VirtAddr) -> EResult<usize> {
	let addr = Process::current().mem_space().brk(addr);
	Ok(addr.0 as _)
}

pub fn mincore(addr: VirtAddr, length: usize, vec: *mut u8) -> EResult<usize> {
	let pages = length.div_ceil(PAGE_SIZE);
	let vec = UserSlice::from_user(vec, pages)?;
	Process::current().mem_space().mincore(addr, pages, vec)?;
	Ok(0)
}

pub fn madvise(_addr: *mut c_void, _length: usize, _advice: c_int) -> EResult<usize> {
	// TODO
	Ok(0)
}

pub fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> EResult<usize> {
	// Check alignment of `addr` and `length`
	if !addr.is_aligned_to(PAGE_SIZE) || len == 0 {
		return Err(errno!(EINVAL));
	}
	let prot = prot as u8;
	Process::current().mem_space().set_prot(addr, len, prot)?;
	Ok(0)
}

pub fn munmap(addr: VirtAddr, length: usize) -> EResult<usize> {
	// Check address alignment
	if !addr.is_aligned_to(PAGE_SIZE) || length == 0 {
		return Err(errno!(EINVAL));
	}
	let pages = length.div_ceil(PAGE_SIZE);
	let length = pages * PAGE_SIZE;
	// Check for overflow
	let Some(end) = addr.0.checked_add(length) else {
		return Err(errno!(EINVAL));
	};
	// Prevent from unmapping kernel memory
	if unlikely(end > memory::PROCESS_END.0) {
		return Err(errno!(EINVAL));
	}
	Process::current()
		.mem_space()
		.unmap(addr, NonZeroUsize::new(pages).unwrap())?;
	Ok(0)
}
