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

use super::mmap;
use core::ffi::{c_int, c_void};
use macros::syscall;
use utils::errno::Errno;

// TODO Check last argument type
#[syscall]
pub fn mmap2(
	addr: *mut c_void,
	length: usize,
	prot: c_int,
	flags: c_int,
	fd: c_int,
	offset: u64,
) -> Result<i32, Errno> {
	mmap::do_mmap(addr, length, prot, flags, fd, offset * 4096)
}
