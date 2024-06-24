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
	process::iovec::IOVec,
	syscall::{Args, SyscallSlice},
};
use core::ffi::c_int;
use utils::errno::{EResult, Errno};

pub fn preadv2(
	Args((fd, iov, iovcnt, offset, flags)): Args<(
		c_int,
		SyscallSlice<IOVec>,
		c_int,
		isize,
		c_int,
	)>,
) -> EResult<usize> {
	super::readv::do_readv(fd, iov, iovcnt, Some(offset), Some(flags))
}
