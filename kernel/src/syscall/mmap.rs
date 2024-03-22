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

//! The `mmap` system call allows the process to allocate memory.

use crate::{
	file::FileType,
	memory,
	process::{mem_space, mem_space::residence::MapResidence, Process},
	syscall::mmap::mem_space::MapConstraint,
};
use core::{
	ffi::{c_int, c_void},
	num::NonZeroUsize,
};
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// Data can be read.
pub const PROT_READ: i32 = 0b001;
/// Data can be written.
pub const PROT_WRITE: i32 = 0b010;
/// Data can be executed.
pub const PROT_EXEC: i32 = 0b100;

/// Changes are shared.
const MAP_SHARED: i32 = 0b001;
/// Interpret addr exactly.
const MAP_FIXED: i32 = 0b010;

/// Converts mmap's `flags` and `prot` to mem space mapping flags.
fn get_flags(flags: i32, prot: i32) -> u8 {
	let mut mem_flags = mem_space::MAPPING_FLAG_USER;

	if flags & MAP_SHARED != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_SHARED;
	}

	if prot & PROT_WRITE != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_WRITE;
	}
	if prot & PROT_EXEC != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_EXEC;
	}

	mem_flags
}

/// Performs the `mmap` system call.
///
/// This function takes a `u64` for `offset` to allow implementing the `mmap2`
/// syscall.
pub fn do_mmap(
	addr: *mut c_void,
	length: usize,
	prot: i32,
	flags: i32,
	fd: i32,
	offset: u64,
) -> EResult<i32> {
	// Check alignment of `addr` and `length`
	if !addr.is_aligned_to(memory::PAGE_SIZE) || length == 0 {
		return Err(errno!(EINVAL));
	}

	// The length in number of pages
	let pages = length.div_ceil(memory::PAGE_SIZE);
	let Some(pages) = NonZeroUsize::new(pages) else {
		return Err(errno!(EINVAL));
	};

	// Check for overflow
	let end = (addr as usize).wrapping_add(pages.get() * memory::PAGE_SIZE);
	if end < addr as usize {
		return Err(errno!(EINVAL));
	}

	let constraint = {
		if !addr.is_null() {
			if flags & MAP_FIXED != 0 {
				MapConstraint::Fixed(addr as _)
			} else {
				MapConstraint::Hint(addr as _)
			}
		} else {
			MapConstraint::None
		}
	};

	// Get the current process
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// The file the mapping points to
	let file_mutex = if fd >= 0 {
		// Check the alignment of the offset
		if offset as usize % memory::PAGE_SIZE != 0 {
			return Err(errno!(EINVAL));
		}

		proc.file_descriptors
			.as_ref()
			.unwrap()
			.lock()
			.get_fd(fd as _)
			.map(|fd| fd.get_open_file().lock().get_file().clone())
	} else {
		None
	};

	// TODO anon flag

	// Get residence
	let residence = match file_mutex {
		Some(file_mutex) => {
			let file = file_mutex.lock();
			// Check the file is suitable
			if !matches!(file.get_type(), FileType::Regular) {
				return Err(errno!(EACCES));
			}
			if prot & PROT_READ != 0 && !proc.access_profile.can_read_file(&file) {
				return Err(errno!(EPERM));
			}
			if prot & PROT_WRITE != 0 && !proc.access_profile.can_write_file(&file) {
				return Err(errno!(EPERM));
			}
			if prot & PROT_EXEC != 0 && !proc.access_profile.can_execute_file(&file) {
				return Err(errno!(EPERM));
			}

			MapResidence::File {
				location: file.get_location().clone(),
				off: offset,
			}
		}
		None => {
			// TODO If the mapping requires a fd, return an error
			MapResidence::Normal
		}
	};

	// The process's memory space
	let mem_space_mutex = proc.get_mem_space().unwrap();
	let mut mem_space = mem_space_mutex.lock();

	let flags = get_flags(flags, prot);

	// The pointer on the virtual memory to the beginning of the mapping
	let result = mem_space.map(constraint, pages, flags, residence.clone());
	match result {
		Ok(ptr) => Ok(ptr as _),
		Err(e) => {
			if constraint != MapConstraint::None {
				let ptr = mem_space.map(MapConstraint::None, pages, flags, residence)?;
				Ok(ptr as _)
			} else {
				Err(e.into())
			}
		}
	}
}

// TODO Check last arg type
#[syscall]
pub fn mmap(
	addr: *mut c_void,
	length: usize,
	prot: c_int,
	flags: c_int,
	fd: c_int,
	offset: u64,
) -> Result<i32, Errno> {
	do_mmap(addr, length, prot, flags, fd, offset as _)
}
