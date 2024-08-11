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
	file::{
		fd::FileDescriptorTable,
		open_file::{OpenFile, O_NONBLOCK},
		FileType,
	},
	limits,
	process::{
		iovec::IOVec,
		mem_space::{copy::SyscallSlice, MemSpace},
		scheduler, Process,
	},
	syscall::{Args, FromSyscallArg},
};
use core::{cmp::min, ffi::c_int};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
	vec,
};
// TODO Handle blocking writes (and thus, EINTR)
// TODO Reimplement by taking example on `writev` (currently doesn't work with blocking files)

/// Reads the given chunks from the file.
///
/// Arguments:
/// - `iov` is the set of chunks
/// - `iovcnt` is the number of chunks in `iov`
/// - `open_file` is the file to read from
fn read(iov: &SyscallSlice<IOVec>, iovcnt: usize, open_file: &mut OpenFile) -> EResult<i32> {
	let mut off = 0;
	let iov = iov.copy_from_user(..iovcnt)?.ok_or(errno!(EFAULT))?;
	for i in iov {
		// Ignore zero entry
		if i.iov_len == 0 {
			continue;
		}
		// The size to read. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, i32::MAX as usize - off);
		let ptr = SyscallSlice::<u8>::from_syscall_arg(i.iov_base as usize);
		// Read
		// TODO perf: do not use a buffer
		let mut buffer = vec![0u8; l]?;
		// FIXME: incorrect. should reuse the same buffer if not full
		let len = open_file.read(0, &mut buffer)?;
		if len == 0 {
			break;
		}
		ptr.copy_to_user(off, &buffer[..(len as usize)])?;
		off += len as usize;
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
	// TODO Handle flags
	let open_file_mutex = fds.lock().get_fd(fd)?.get_open_file().clone();
	// Validation
	let (start_off, update_off) = match offset {
		Some(o @ 0..) => (o as u64, false),
		None | Some(-1) => {
			let open_file = open_file_mutex.lock();
			(open_file.get_offset(), true)
		}
		Some(..-1) => return Err(errno!(EINVAL)),
	};
	let file_type = open_file_mutex.lock().get_file().lock().stat.file_type;
	if file_type == FileType::Link {
		return Err(errno!(EINVAL));
	}
	let mut open_file = open_file_mutex.lock();
	let flags = open_file.get_flags();
	// Change the offset temporarily
	let prev_off = open_file.get_offset();
	open_file.set_offset(start_off);
	let len = read(&iov, iovcnt as _, &mut open_file)?;
	// Restore previous offset
	if !update_off {
		open_file.set_offset(prev_off);
	}
	if len == 0 && flags & O_NONBLOCK != 0 {
		// The file descriptor is non-blocking
		return Err(errno!(EAGAIN));
	}
	Ok(len as _)
}

pub fn readv(
	Args((fd, iov, iovcnt)): Args<(c_int, SyscallSlice<IOVec>, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_readv(fd, iov, iovcnt, None, None, fds)
}
