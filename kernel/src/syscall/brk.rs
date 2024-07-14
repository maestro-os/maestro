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

//! The `brk` system call allows to displace the end of the data segment of the
//! process, thus allowing memory allocations.

use crate::{
	process::{mem_space::MemSpace, Process},
	syscall::Args,
};
use core::ffi::c_void;
use utils::{
	errno::{EResult, Errno},
	lock::IntMutex,
	ptr::arc::Arc,
};

pub fn brk(
	Args(addr): Args<*mut c_void>,
	mem_space_mutex: Arc<IntMutex<MemSpace>>,
) -> EResult<usize> {
	let mut mem_space = mem_space_mutex.lock();
	let old = mem_space.get_brk_ptr();
	if mem_space.set_brk_ptr(addr).is_ok() {
		Ok(addr as _)
	} else {
		Ok(old as _)
	}
}
