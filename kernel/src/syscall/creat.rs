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
use crate::{
	file,
	file::{O_CREAT, O_TRUNC, O_WRONLY},
	memory::user::UserString,
	syscall::{openat::do_openat, util::at::AT_FDCWD},
};
use core::ffi::c_int;
use utils::errno::EResult;

pub fn creat(Args((pathname, mode)): Args<(UserString, c_int)>) -> EResult<usize> {
	do_openat(AT_FDCWD, pathname, O_CREAT | O_WRONLY | O_TRUNC, mode as _)
}
