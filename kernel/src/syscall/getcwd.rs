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

//! The getcwd system call allows to retrieve the current working directory of
//! the current process.

use crate::process::{mem_space::ptr::SyscallSlice, Process};
use macros::syscall;
use utils::{errno, errno::Errno, format};

#[syscall]
pub fn getcwd(buf: SyscallSlice<u8>, size: usize) -> Result<i32, Errno> {
	if size == 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let cwd = format!("{}", proc.cwd.0)?;

	// Checking that the buffer is large enough
	if size < cwd.len() + 1 {
		return Err(errno!(ERANGE));
	}

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	let cwd_slice = cwd.as_bytes();
	let buf_slice = buf
		.get_mut(&mut mem_space_guard, size as _)?
		.ok_or_else(|| errno!(EINVAL))?;
	utils::slice_copy(cwd_slice, buf_slice);
	buf_slice[cwd.len()] = b'\0';

	Ok(buf.as_ptr() as _)
}
