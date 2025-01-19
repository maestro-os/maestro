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

//! The `writev` system call allows to write sparse data on a file descriptor.

use crate::{
	file::{fd::FileDescriptorTable, File, FileType, O_NONBLOCK},
	process::{
		mem_space::{
			copy::{SyscallIOVec, SyscallSlice},
			MemSpace,
		},
		scheduler,
		signal::Signal,
		Process,
	},
	sync::mutex::Mutex,
	syscall::{Args, FromSyscallArg},
};
use core::{cmp::min, ffi::c_int, sync::atomic};
use utils::{
	errno,
	errno::{EResult, Errno},
	limits::IOV_MAX,
	ptr::arc::Arc,
};

// FIXME: the operation has to be atomic

/// Writes the given chunks to the file.
///
/// Arguments:
/// - `iov` is the set of chunks
/// - `iovcnt` is the number of chunks in `iov`
/// - `offset` is the offset at which the write operation in the file begins
/// - `file` is the file to write to
fn write(iov: SyscallIOVec, iovcnt: usize, offset: Option<u64>, file: &File) -> EResult<usize> {
	let mut off = 0;
	for i in iov.iter(iovcnt) {
		let i = i?;
		// The size to write. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, i32::MAX as usize - off);
		let ptr = SyscallSlice::<u8>::from_ptr(i.iov_base as usize);
		if let Some(buf) = ptr.copy_from_user_vec(0, l)? {
			let len = if let Some(offset) = offset {
				let file_off = offset + off as u64;
				file.ops.write(file, file_off, &buf)?
			} else {
				let off = file.off.load(atomic::Ordering::Acquire);
				let len = file.ops.write(file, off, &buf)?;
				// Update offset
				let new_off = off.saturating_add(len as u64);
				file.off.store(new_off, atomic::Ordering::Release);
				len
			};
			off += len;
		}
	}
	Ok(off)
}

/// Performs the `writev` operation.
///
/// Arguments:
/// - `fd` is the file descriptor
/// - `iov` the IO vector
/// - `iovcnt` the number of entries in the IO vector
/// - `offset` is the offset in the file
/// - `flags` is the set of flags
pub fn do_writev(
	fd: i32,
	iov: SyscallIOVec,
	iovcnt: i32,
	offset: Option<isize>,
	_flags: Option<i32>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if iovcnt < 0 || iovcnt as usize > IOV_MAX {
		return Err(errno!(EINVAL));
	}
	let offset = match offset {
		Some(o @ 0..) => Some(o as u64),
		None | Some(-1) => None,
		Some(..-1) => return Err(errno!(EINVAL)),
	};
	// Get file
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	write(iov, iovcnt as _, offset, &file)
}

pub fn writev(
	Args((fd, iov, iovcnt)): Args<(c_int, SyscallIOVec, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_writev(fd, iov, iovcnt, None, None, fds)
}
