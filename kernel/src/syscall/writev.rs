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
	limits,
	process::{
		iovec::IOVec,
		mem_space::{copy::SyscallSlice, MemSpace},
		scheduler,
		signal::Signal,
		Process,
	},
	syscall::{Args, FromSyscallArg},
};
use core::{cmp::min, ffi::c_int, sync::atomic};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};
// TODO Handle blocking writes (and thus, EINTR)

/// Writes the given chunks to the file.
///
/// Arguments:
/// - `iov` is the set of chunks
/// - `iovcnt` is the number of chunks in `iov`
/// - `offset` is the offset at which the write operation in the file begins
/// - `file` is the file to write to
fn write(
	iov: &SyscallSlice<IOVec>,
	iovcnt: usize,
	offset: Option<u64>,
	file: &File,
) -> EResult<usize> {
	let mut off = 0;
	let iov = iov.copy_from_user(..iovcnt)?.ok_or(errno!(EFAULT))?;
	for i in iov {
		// The size to write. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, usize::MAX - off);
		let ptr = SyscallSlice::<u8>::from_syscall_arg(i.iov_base as usize);
		if let Some(buf) = ptr.copy_from_user(..l)? {
			// FIXME: if not everything has been written, must retry with the same buffer with the
			// corresponding offset
			let len = if let Some(offset) = offset {
				let file_off = offset + off as u64;
				file.write(file_off, &buf)?
			} else {
				let off = file.off.load(atomic::Ordering::Acquire);
				let len = file.write(off, &buf)?;
				file.off.fetch_add(len as u64, atomic::Ordering::Release);
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
	iov: SyscallSlice<IOVec>,
	iovcnt: i32,
	offset: Option<isize>,
	_flags: Option<i32>,
	proc: Arc<IntMutex<Process>>,
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
	// Get file
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	let len = match write(&iov, iovcnt as _, offset, &file) {
		Ok(len) => len,
		Err(e) => {
			// If writing to a broken pipe, kill with SIGPIPE
			if e.as_int() == errno::EPIPE {
				let mut proc = proc.lock();
				proc.kill_now(Signal::SIGPIPE);
			}
			return Err(e);
		}
	};
	Ok(len)
}

pub fn writev(
	Args((fd, iov, iovcnt)): Args<(c_int, SyscallSlice<IOVec>, c_int)>,
	proc: Arc<IntMutex<Process>>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_writev(fd, iov, iovcnt, None, None, proc, fds)
}
