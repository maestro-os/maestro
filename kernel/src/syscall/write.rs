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

//! The `write` system call, which allows to write data to a file.

use super::Args;
use crate::{
	file::{fd::FileDescriptorTable, open_file::O_NONBLOCK, FileType},
	idt,
	process::{mem_space::copy::SyscallSlice, regs::Regs, scheduler, Process},
	syscall::Signal,
};
use core::{cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
	io,
	io::IO,
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};
// TODO O_ASYNC

pub fn write(
	Args((fd, buf, count)): Args<(c_int, SyscallSlice<u8>, usize)>,
	regs: &Regs,
	fds: Arc<Mutex<FileDescriptorTable>>,
	proc: &IntMutex<Process>,
) -> EResult<usize> {
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let open_file = fds.lock().get_fd(fd)?.get_open_file().clone();
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
			// TODO find a way to avoid allocating here
			let buf_slice = buf.copy_from_user(..len)?.ok_or(errno!(EFAULT))?;
			// Write file
			let mut open_file = open_file.lock();
			let flags = open_file.get_flags();
			let len = match open_file.write(0, &buf_slice) {
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
				// The file descriptor is non-blocking
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
