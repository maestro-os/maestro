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

//! Filesystem synchronization system calls.

use crate::{
	file::{fd::FileDescriptorTable, vfs},
	memory::VirtAddr,
	process::mem_space::MemSpace,
	sync::mutex::{IntMutex, Mutex},
	syscall::Args,
};
use core::{ffi::c_int, intrinsics::unlikely};
use utils::{errno, errno::EResult, limits::PAGE_SIZE, ptr::arc::Arc};

/// Schedules a synchronization and returns directly
const MS_ASYNC: i32 = 0b001;
/// Synchronizes the mapping before returning
const MS_SYNC: i32 = 0b010;
/// Invalidates other mappings of the same file, so they can be updated
const MS_INVALIDATE: i32 = 0b100;

pub fn sync() -> EResult<usize> {
	// TODO sync all files on the VFS
	Ok(0)
}

pub fn syncfs(Args(fd): Args<c_int>, fds: Arc<Mutex<FileDescriptorTable>>) -> EResult<usize> {
	let fds = fds.lock();
	if fd < 0 {
		return Err(errno!(EBADF));
	}
	let file = fds.get_fd(fd)?.get_file();
	let Some(ent) = &file.vfs_entry else {
		return Ok(0);
	};
	let _fs = &ent.node().fs;
	// TODO sync all files on the filesystem
	Ok(0)
}

pub fn fsync(Args(fd): Args<c_int>, fds: Arc<Mutex<FileDescriptorTable>>) -> EResult<usize> {
	let fds = fds.lock();
	if fd < 0 {
		return Err(errno!(EBADF));
	}
	let file = fds.get_fd(fd)?.get_file();
	if let Some(node) = file.node() {
		node.sync(.., true)?;
	}
	Ok(0)
}

pub fn msync(
	Args((addr, length, flags)): Args<(VirtAddr, usize, c_int)>,
	mem_space: Arc<IntMutex<MemSpace>>,
) -> EResult<usize> {
	// Check address alignment
	if !addr.is_aligned_to(PAGE_SIZE) {
		return Err(errno!(EINVAL));
	}
	// Check for conflicts in flags
	if unlikely(flags & MS_ASYNC != 0 && flags & MS_SYNC != 0) {
		return Err(errno!(EINVAL));
	}
	// Iterate over mappings
	let mem_space = mem_space.lock();
	let mut i = 0;
	let pages = length.div_ceil(PAGE_SIZE);
	while i < pages {
		let mapping = mem_space.get_mapping_for_addr(addr).ok_or(errno!(ENOMEM))?;
		mapping.sync(&mem_space.vmem)?; // TODO Use flags
		i += mapping.get_size().get();
	}
	Ok(0)
}
