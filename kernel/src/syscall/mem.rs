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
	file::{FileType, fd::FileDescriptorTable, perm::AccessProfile},
	memory,
	memory::VirtAddr,
	process::{
		mem_space,
		mem_space::{MAP_ANONYMOUS, MAP_FIXED, MemSpace, PROT_EXEC, PROT_READ, PROT_WRITE},
	},
	sync::mutex::Mutex,
	syscall::{Args, mem::mem_space::MapConstraint},
};
use core::{
	ffi::{c_int, c_void},
	intrinsics::unlikely,
	num::NonZeroUsize,
};
use utils::{errno, errno::EResult, limits::PAGE_SIZE, ptr::arc::Arc};

/// Performs the `mmap` system call.
#[allow(clippy::too_many_arguments)]
pub fn do_mmap(
	addr: VirtAddr,
	length: usize,
	prot: i32,
	flags: i32,
	fd: i32,
	offset: u64,
	fds: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
	mem_space: Arc<MemSpace>,
) -> EResult<usize> {
	// Check alignment of `addr` and `length`
	if !addr.is_aligned_to(PAGE_SIZE) || length == 0 {
		return Err(errno!(EINVAL));
	}
	// The length in number of pages
	let pages = length.div_ceil(PAGE_SIZE);
	let Some(pages) = NonZeroUsize::new(pages) else {
		return Err(errno!(EINVAL));
	};
	// Check for overflow
	if unlikely(addr.0.checked_add(pages.get() * PAGE_SIZE).is_none()) {
		return Err(errno!(EINVAL));
	}
	let prot = prot as u8;
	let flags = flags as u8;
	let constraint = {
		if !addr.is_null() {
			if flags & MAP_FIXED != 0 {
				MapConstraint::Fixed(addr)
			} else {
				MapConstraint::Hint(addr)
			}
		} else {
			MapConstraint::None
		}
	};
	let file = if flags & MAP_ANONYMOUS == 0 {
		// Validation
		if unlikely(fd < 0) {
			return Err(errno!(EBADF));
		}
		if unlikely(offset as usize % PAGE_SIZE != 0) {
			return Err(errno!(EINVAL));
		}
		// Get file
		let file = fds.lock().get_fd(fd)?.get_file().clone();
		// Check permissions
		let stat = file.stat()?;
		if stat.get_type() != Some(FileType::Regular) {
			return Err(errno!(EACCES));
		}
		if prot & PROT_READ != 0 && !ap.can_read_file(&stat) {
			return Err(errno!(EPERM));
		}
		if prot & PROT_WRITE != 0 && !ap.can_write_file(&stat) {
			return Err(errno!(EPERM));
		}
		if prot & PROT_EXEC != 0 && !ap.can_execute_file(&stat) {
			return Err(errno!(EPERM));
		}
		Some(file)
	} else {
		None
	};
	// The pointer on the virtual memory to the beginning of the mapping
	let result = mem_space.map(constraint, pages, prot, flags, file.clone(), offset);
	match result {
		Ok(ptr) => Ok(ptr as _),
		Err(e) => {
			if constraint != MapConstraint::None {
				let ptr = mem_space.map(MapConstraint::None, pages, prot, flags, file, offset)?;
				Ok(ptr as _)
			} else {
				Err(e)
			}
		}
	}
}

pub fn mmap(
	Args((addr, length, prot, flags, fd, offset)): Args<(
		VirtAddr,
		usize,
		c_int,
		c_int,
		c_int,
		u64,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
	mem_space: Arc<MemSpace>,
) -> EResult<usize> {
	do_mmap(
		addr,
		length,
		prot,
		flags,
		fd,
		offset as _,
		fds,
		ap,
		mem_space,
	)
}

pub fn mmap2(
	Args((addr, length, prot, flags, fd, offset)): Args<(
		VirtAddr,
		usize,
		c_int,
		c_int,
		c_int,
		u64,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
	mem_space: Arc<MemSpace>,
) -> EResult<usize> {
	do_mmap(
		addr,
		length,
		prot,
		flags,
		fd,
		offset * 4096,
		fds,
		ap,
		mem_space,
	)
}

pub fn madvise(
	Args((_addr, _length, _advice)): Args<(*mut c_void, usize, c_int)>,
) -> EResult<usize> {
	// TODO
	Ok(0)
}

pub fn mprotect(
	Args((addr, len, prot)): Args<(*mut c_void, usize, c_int)>,
	mem_space: Arc<MemSpace>,
	ap: AccessProfile,
) -> EResult<usize> {
	// Check alignment of `addr` and `length`
	if !addr.is_aligned_to(PAGE_SIZE) || len == 0 {
		return Err(errno!(EINVAL));
	}
	let prot = prot as u8;
	mem_space.set_prot(addr, len, prot, &ap)?;
	Ok(0)
}

pub fn munmap(
	Args((addr, length)): Args<(VirtAddr, usize)>,
	mem_space: Arc<MemSpace>,
) -> EResult<usize> {
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
	mem_space.unmap(addr, NonZeroUsize::new(pages).unwrap())?;
	Ok(0)
}
