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
	file::{fd::FileDescriptorTable, FileType},
	idt,
	process::{mem_space::copy::SyscallSlice, regs::Regs, scheduler, Process},
	syscall::Signal,
};
use core::{cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};

// TODO O_ASYNC

pub fn write(
	Args((fd, buf, count)): Args<(c_int, SyscallSlice<u8>, usize)>,
	proc: Arc<IntMutex<Process>>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	let len = min(count, usize::MAX);
	if len == 0 {
		return Ok(0);
	}
	let file_mutex = fds.lock().get_fd(fd)?.get_file().clone();
	let mut file = file_mutex.lock();
	// Validation
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// TODO determine why removing this causes a deadlock
	cli();
	// TODO find a way to avoid allocating here
	let buf_slice = buf.copy_from_user(..len)?.ok_or(errno!(EFAULT))?;
	// Write file
	let res = file
		.ops()
		.write_content(file.get_location(), file.off, &buf_slice);
	match res {
		Ok(len) => {
			file.off += len as u64;
			Ok(len)
		}
		Err(e) => {
			// If writing to a broken pipe, kill with SIGPIPE
			if e.as_int() == errno::EPIPE {
				let mut proc = proc.lock();
				proc.kill_now(Signal::SIGPIPE);
			}
			Err(e)
		}
	}
}
