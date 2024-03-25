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

//! The `fadvise64_64` syscall gives hints to the kernel about file accesses.

use core::ffi::c_int;
use macros::syscall;
use utils::errno::Errno;

// TODO Check args type
#[syscall]
pub fn fadvise64_64(_fd: c_int, _offset: u64, _len: u64, _advice: c_int) -> Result<i32, Errno> {
	// TODO
	Ok(0)
}
