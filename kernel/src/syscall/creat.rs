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

//! The `creat` system call allows to create and open a file.

use super::{open, Args};
use crate::{file, process::mem_space::copy::SyscallString};
use core::ffi::c_int;
use utils::errno::EResult;

// TODO Check args type
pub fn creat(Args((pathname, mode)): Args<(SyscallString, c_int)>) -> EResult<usize> {
	let flags = file::O_CREAT | file::O_WRONLY | file::O_TRUNC;
	open::open_(pathname, flags, mode as _)
}
