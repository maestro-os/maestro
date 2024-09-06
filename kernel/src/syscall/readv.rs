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

//! The `readv` system call allows to read from file descriptor and write it into a sparse buffer.

use crate::{
	file::{fd::FileDescriptorTable, File, FileType},
	limits,
	process::{
		iovec::IOVec,
		mem_space::{copy::SyscallSlice, MemSpace},
		scheduler, Process,
	},
	syscall::{Args, FromSyscallArg},
};
use core::{cmp::min, ffi::c_int, sync::atomic};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
	vec,
};

/// Reads the given chunks from the file.
///
/// Arguments:
/// - `iov` is the set of chunks
/// - `iovcnt` is the number of chunks in `iov`
/// - `offset` is the offset at which the read operation in the file begins
/// - `open_file` is the file to read from
fn read(
	iov: &SyscallSlice<IOVec>,
	iovcnt: usize,
	offset: Option<u64>,
	file: &File,
) -> EResult<usize> {
	let mut off = 0;
	let iov = iov.copy_from_user(..iovcnt)?.ok_or(errno!(EFAULT))?;
	for i in iov {
		// The size to read. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, usize::MAX - off);
		let ptr = SyscallSlice::<u8>::from_syscall_arg(i.iov_base as usize);
		// Read
		// TODO perf: do not use a buffer
		let mut buf = vec![0u8; l]?;
		let mut inner_off = 0;
		while inner_off < buf.len() {
			let len = if let Some(offset) = offset {
				let file_off = offset + off as u64;
				file.ops.read(file, file_off, &mut buf)?
			} else {
				let off = file.off.load(atomic::Ordering::Acquire);
				let len = file.ops.read(file, off, &mut buf)?;
				// Update offset
				let new_off = off.saturating_add(len as u64);
				file.off.store(new_off, atomic::Ordering::Release);
				len
			};
			if len == 0 {
				break;
			}
			inner_off += len;
		}
		ptr.copy_to_user(off, &buf[..inner_off])?;
		off += inner_off;
		// If the last buffer reached the end, stop
		if inner_off < l {
			break;
		}
	}
	Ok(off as _)
}

/// Performs the readv operation.
///
/// Arguments:
/// - `fd` is the file descriptor
/// - `iov` the IO vector
/// - `iovcnt` the number of entries in the IO vector
/// - `offset` is the offset in the file
/// - `flags` is the set of flags
pub fn do_readv(
	fd: c_int,
	iov: SyscallSlice<IOVec>,
	iovcnt: c_int,
	offset: Option<isize>,
	_flags: Option<i32>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if iovcnt < 0 || iovcnt as usize > limits::IOV_MAX {
		return Err(errno!(EINVAL));
	}
	let offset = match offset {
		Some(o @ 0..) => Some(o as u64),
		None | Some(-1) => None,
		Some(..-1) => return Err(errno!(EINVAL)),
	};
	// TODO Handle flags
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	let len = read(&iov, iovcnt as _, offset, &file)?;
	Ok(len as _)
}

pub fn readv(
	Args((fd, iov, iovcnt)): Args<(c_int, SyscallSlice<IOVec>, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_readv(fd, iov, iovcnt, None, None, fds)
}
