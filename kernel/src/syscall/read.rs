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

//! The read system call allows to read the content of an open file.

use super::Args;
use crate::{
	file::{open_file::O_NONBLOCK, FileType},
	process::{mem_space::copy::SyscallSlice, regs::Regs, scheduler, Process},
};
use core::{cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
	io,
	io::IO,
	vec,
};

// TODO O_ASYNC

pub fn read(
	Args((fd, buf, count)): Args<(c_int, SyscallSlice<u8>, usize)>,
	regs: &Regs,
) -> EResult<usize> {
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let (proc, open_file) = {
		let proc_mutex = Process::current();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.file_descriptors.clone().unwrap();
		let fds = fds_mutex.lock();
		let open_file_mutex = fds.get_fd(fd)?.get_open_file().clone();

		drop(proc);
		(proc_mutex, open_file_mutex)
	};
	// Validation
	let file_type = open_file.lock().get_file().lock().stat.file_type;
	if file_type == FileType::Link {
		return Err(errno!(EINVAL));
	}
	loop {
		super::util::handle_signal(regs);

		{
			// TODO determine why removing this causes a deadlock
			cli();
			// TODO perf: a buffer is not necessarily required
			let mut buffer = vec![0u8; count]?;

			// Read file
			let mut open_file = open_file.lock();
			let flags = open_file.get_flags();
			let (len, eof) = open_file.read(0, &mut buffer)?;

			// Write back
			buf.copy_to_user(&buffer[..(len as usize)])?;

			if len == 0 && eof {
				return Ok(0);
			}
			if len > 0 || flags & O_NONBLOCK != 0 {
				// The file descriptor is non-blocking
				return Ok(len as _);
			}

			// Block on file
			let mut proc = proc.lock();
			open_file.add_waiting_process(&mut proc, io::POLLIN | io::POLLERR)?;
		}

		// Make current process sleep
		scheduler::end_tick();
	}
}
