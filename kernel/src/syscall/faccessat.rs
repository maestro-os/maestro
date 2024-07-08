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

//! The `faccessat` system call allows to check access to a given file.

use crate::{process::mem_space::copy::SyscallString, syscall::Args};
use core::ffi::c_int;
use utils::errno::EResult;

pub fn faccessat(
	Args((dir_fd, pathname, mode)): Args<(c_int, SyscallString, c_int)>,
) -> EResult<usize> {
	super::access::do_access(Some(dir_fd), pathname, mode, None)
}
