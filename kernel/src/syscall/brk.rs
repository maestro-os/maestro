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
	memory::VirtAddr,
	process::{Process, mem_space::MemSpace},
	sync::mutex::IntMutex,
	syscall::Args,
};
use core::ffi::c_void;
use utils::{
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn brk(Args(addr): Args<VirtAddr>, mem_space: Arc<MemSpace>) -> EResult<usize> {
	let addr = mem_space.brk(addr);
	Ok(addr.0 as _)
}
