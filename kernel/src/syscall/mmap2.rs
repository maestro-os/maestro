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

//! The `mmap2` system call is similar to `mmap`, except it takes a file offset
//! in pages.

use super::{mmap, Args};
use crate::{
	file::{fd::FileDescriptorTable, perm::AccessProfile},
	memory::VirtAddr,
	process::mem_space::MemSpace,
};
use core::ffi::{c_int, c_void};
use utils::{
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};

pub fn mmap2(
	Args((addr, length, prot, flags, fd, offset)): Args<(
		VirtAddr,
		usize,
		c_int,
		c_int,
		c_int,
		u64,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
	mem_space: Arc<IntMutex<MemSpace>>,
) -> EResult<usize> {
	mmap::do_mmap(
		addr,
		length,
		prot,
		flags,
		fd,
		offset * 4096,
		fds,
		ap,
		mem_space,
	)
}
