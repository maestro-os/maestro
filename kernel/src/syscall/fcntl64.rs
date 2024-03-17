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

//! The `fcntl64` syscall call allows to manipulate a file descriptor.

use core::ffi::{c_int, c_void};
use macros::syscall;
use utils::errno::Errno;

#[syscall]
pub fn fcntl64(fd: c_int, cmd: c_int, arg: *mut c_void) -> Result<i32, Errno> {
	super::fcntl::do_fcntl(fd, cmd, arg, true)
}
