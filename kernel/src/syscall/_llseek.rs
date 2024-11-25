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

//! The `_llseek` system call repositions the offset of a file descriptor.

use crate::{
	file::fd::FileDescriptorTable,
	process::{
		mem_space::{copy::SyscallPtr, MemSpace},
		Process,
	},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{
	ffi::{c_uint, c_ulong},
	sync::atomic,
};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

/// Sets the offset from the given value.
const SEEK_SET: u32 = 0;
/// Sets the offset relative to the current offset.
const SEEK_CUR: u32 = 1;
/// Sets the offset relative to the end of the file.
const SEEK_END: u32 = 2;

pub fn _llseek(
	Args((fd, offset_high, offset_low, result, whence)): Args<(
		c_uint,
		u32,
		u32,
		SyscallPtr<u64>,
		c_uint,
	)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let fds = fds_mutex.lock();
	let file = fds.get_fd(fd as _)?.get_file();
	// Compute the offset
	let off = ((offset_high as u64) << 32) | (offset_low as u64);
	let base = match whence {
		SEEK_SET => 0,
		SEEK_CUR => file.off.load(atomic::Ordering::Acquire),
		SEEK_END => file.stat()?.size,
		_ => return Err(errno!(EINVAL)),
	};
	let off = base.checked_add(off).ok_or_else(|| errno!(EOVERFLOW))?;
	// Write the result to the userspace
	result.copy_to_user(&off)?;
	// Set the new offset
	file.off.store(off, atomic::Ordering::Release);
	Ok(0)
}
