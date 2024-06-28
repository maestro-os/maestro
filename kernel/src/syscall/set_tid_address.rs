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

//! The `set_tid_address` system call sets the `clear_child_tid` attribute with
//! the given pointer.

use crate::{
	process::Process,
	syscall::{Args, SyscallPtr},
};
use core::{ffi::c_int, ptr::NonNull};
use utils::errno::{EResult, Errno};

pub fn set_tid_address(Args(tidptr): Args<SyscallPtr<c_int>>) -> EResult<usize> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();
	proc.clear_child_tid = tidptr.0;

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();
	// Set the TID at pointer if accessible
	if let Some(tidptr) = tidptr.get_mut(&mut mem_space_guard)? {
		*tidptr = proc.tid as _;
	}

	Ok(proc.tid as _)
}
