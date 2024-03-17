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

//! The `pwritev2` system call allows to write sparse data on a file descriptor.

use crate::process::{iovec::IOVec, mem_space::ptr::SyscallSlice};
use core::ffi::c_int;
use macros::syscall;
use utils::errno::Errno;

#[syscall]
pub fn pwritev2(
	fd: c_int,
	iov: SyscallSlice<IOVec>,
	iovcnt: c_int,
	offset: isize,
	flags: c_int,
) -> Result<i32, Errno> {
	super::writev::do_writev(fd, iov, iovcnt, Some(offset), Some(flags))
}
