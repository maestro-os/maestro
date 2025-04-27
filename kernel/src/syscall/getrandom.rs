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

//! The `getrandom` system call allows to get random bytes.

use crate::{crypto::rand, memory::user::UserSlice, process::Process, syscall::Args};
use core::ffi::c_uint;
use utils::{
	errno,
	errno::{EResult, Errno},
	vec,
};

pub fn getrandom(Args((buf, buflen, flags)): Args<(*mut u8, usize, c_uint)>) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, buflen)?;
	rand::getrandom(buf, flags)
}
