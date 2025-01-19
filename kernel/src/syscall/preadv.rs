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

//! The `readv` system call allows to read from file descriptor and write it into a sparse buffer.

use crate::{
	file::fd::FileDescriptorTable,
	process::{
		mem_space::copy::{SyscallIOVec, SyscallSlice},
		Process,
	},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{errno::EResult, ptr::arc::Arc};

pub fn preadv(
	Args((fd, iov, iovcnt, offset_low, offset_high)): Args<(
		c_int,
		SyscallIOVec,
		c_int,
		isize,
		isize,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	#[allow(arithmetic_overflow)]
	let offset = offset_low | (offset_high << 32);
	super::readv::do_readv(fd, iov, iovcnt, Some(offset), None, fds)
}
