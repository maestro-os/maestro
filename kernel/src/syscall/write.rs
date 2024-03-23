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

//! This module implements the `write` system call, which allows to write data
//! to a file.

use crate::{
	file::open_file::O_NONBLOCK,
	process::{mem_space::ptr::SyscallSlice, scheduler, Process},
	syscall::Signal,
};
use core::{cmp::min, ffi::c_int};
use macros::syscall;
use utils::{errno, errno::Errno, io, io::IO};

// TODO O_ASYNC

#[syscall]
pub fn write(fd: c_int, buf: SyscallSlice<u8>, count: usize) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}

	let (proc, mem_space, open_file) = {
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

	loop {
		super::util::handle_signal(regs);

		{
			let mem_space_guard = mem_space.lock();
			let buf_slice = buf.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

			// Write file
			let mut open_file = open_file.lock();
			let flags = open_file.get_flags();
			let len = match open_file.write(0, buf_slice) {
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
