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
	file::open_file::{OpenFile, O_NONBLOCK},
	limits,
	process::{
		iovec::IOVec,
		mem_space::{ptr::SyscallSlice, MemSpace},
		scheduler,
		signal::Signal,
		Process,
	},
};
use core::{cmp::min, ffi::c_int};
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
	io,
	io::IO,
};

// TODO Handle blocking writes (and thus, EINTR)

/// Writes the given chunks to the file.
///
/// Arguments:
/// - `mem_space` is the memory space of the current process
/// - `iov` is the set of chunks
/// - `iovcnt` is the number of chunks in `iov`
/// - `open_file` is the file to write to
fn write(
	mem_space: &MemSpace,
	iov: &SyscallSlice<IOVec>,
	iovcnt: usize,
	open_file: &mut OpenFile,
) -> EResult<i32> {
	let iov = iov.get(mem_space, iovcnt)?.ok_or(errno!(EFAULT))?;
	let mut total_len = 0;

	for i in iov {
		// Ignore zero entry
		if i.iov_len == 0 {
			continue;
		}

		// The size to write. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, i32::MAX as usize - total_len);
		let ptr = SyscallSlice::<u8>::from(i.iov_base as usize);

		if let Some(slice) = ptr.get(mem_space, l)? {
			// The offset is ignored
			total_len += open_file.write(0, slice)? as usize;
		}
	}

	Ok(total_len as _)
}

/// Peforms the writev operation.
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
) -> EResult<i32> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}
	if iovcnt < 0 || iovcnt as usize > limits::IOV_MAX {
		return Err(errno!(EINVAL));
	}

	let (proc, mem_space, open_file_mutex) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap().clone();

		let fds_mutex = proc.file_descriptors.clone().unwrap();
		let fds = fds_mutex.lock();
		let open_file_mutex = fds
			.get_fd(fd as _)
			.ok_or(errno!(EBADF))?
			.get_open_file()
			.clone();

		drop(proc);
		(proc_mutex, mem_space, open_file_mutex)
	};

	let (start_off, update_off) = match offset {
		Some(o @ 0..) => (o as u64, false),
		None | Some(-1) => {
			let open_file = open_file_mutex.lock();
			(open_file.get_offset(), true)
		}
		Some(..-1) => return Err(errno!(EINVAL)),
	};

	loop {
		// TODO super::util::signal_check(regs);

		{
			let mut open_file = open_file_mutex.lock();
			let flags = open_file.get_flags();

			// Change the offset temporarily
			let prev_off = open_file.get_offset();
			open_file.set_offset(start_off);

			let mem_space_guard = mem_space.lock();
			let len = match write(&mem_space_guard, &iov, iovcnt as _, &mut open_file) {
				Ok(len) => len,
				Err(e) => {
					// If writing to a broken pipe, kill with SIGPIPE
					if e.as_int() == errno::EPIPE {
						let mut proc = proc.lock();
						proc.kill_now(&Signal::SIGPIPE);
					}
					return Err(e);
				}
			};

			// Restore previous offset
			if !update_off {
				open_file.set_offset(prev_off);
			}

			if len > 0 {
				return Ok(len as _);
			}
			if flags & O_NONBLOCK != 0 {
				// The file descriptor is non blocking
				return Err(errno!(EAGAIN));
			}

			// Block on file
			let mut proc = proc.lock();
			open_file.add_waiting_process(&mut proc, io::POLLOUT | io::POLLERR)?;
		}

		// Make current process sleep
		scheduler::end_tick();
	}
}

#[syscall]
pub fn writev(fd: c_int, iov: SyscallSlice<IOVec>, iovcnt: c_int) -> EResult<i32> {
	do_writev(fd, iov, iovcnt, None, None)
}
