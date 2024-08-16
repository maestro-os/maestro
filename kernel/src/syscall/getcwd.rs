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

//! The `getcwd` system call allows to retrieve the current working directory of
//! the current process.

use crate::{
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use utils::{
	errno,
	errno::{EResult, Errno},
	format,
	lock::{IntMutex, IntMutexGuard},
	ptr::arc::Arc,
};

pub fn getcwd(
	Args((buf, size)): Args<(SyscallSlice<u8>, usize)>,
	proc: Arc<IntMutex<Process>>,
) -> EResult<usize> {
	let proc = proc.lock();
	let cwd = proc.cwd.lock();
	let path = cwd.get_path();
	// Check that the buffer is large enough
	if size < path.len() + 1 {
		return Err(errno!(ERANGE));
	}
	// Write
	let cwd = path.as_bytes();
	buf.copy_to_user(0, cwd)?;
	buf.copy_to_user(cwd.len(), b"\0")?;
	Ok(buf.as_ptr() as _)
}
