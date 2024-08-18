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

//! The `read` system call allows to read the content of an open file.

use super::Args;
use crate::{
	file::{fd::FileDescriptorTable, FileType},
	process::{mem_space::copy::SyscallSlice, regs::Regs, scheduler, Process},
};
use core::{cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
	vec,
};
// TODO O_ASYNC

pub fn read(
	Args((fd, buf, count)): Args<(c_int, SyscallSlice<u8>, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	let len = min(count, usize::MAX);
	if len == 0 {
		return Ok(0);
	}
	let file_mutex = fds.lock().get_fd(fd)?.get_file().clone();
	let file = file_mutex.lock();
	// Validation
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// TODO determine why removing this causes a deadlock
	cli();
	// TODO perf: a buffer is not necessarily required
	let mut buffer = vec![0u8; count]?;
	// Read file
	let mut file = file_mutex.lock();
	let len = file.read(file.off, &mut buffer)?;
	file.off += len as u64;
	// Write back
	buf.copy_to_user(0, &buffer[..len])?;
	Ok(len as _)
}
