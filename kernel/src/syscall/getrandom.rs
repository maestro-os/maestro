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

use crate::{
	crypto::rand,
	process::{mem_space::copy::UserSlice, Process},
	syscall::Args,
};
use core::ffi::c_uint;
use utils::{
	errno,
	errno::{EResult, Errno},
	vec,
};

/// If set, bytes are drawn from the randomness source instead of `urandom`.
pub const GRND_RANDOM: u32 = 2;
/// If set, the function does not block. If no entropy is available, the function
/// returns [`EAGAIN`].
pub const GRND_NONBLOCK: u32 = 1;

/// Performs the `getrandom` system call.
pub fn do_getrandom(buf: UserSlice<u8>, flags: c_uint) -> EResult<usize> {
	let mut pool = rand::ENTROPY_POOL.lock();
	let Some(pool) = &mut *pool else {
		return Ok(0);
	};
	pool.read(buf, flags & GRND_RANDOM != 0, flags & GRND_NONBLOCK != 0)
}

pub fn getrandom(Args((buf, buflen, flags)): Args<(*mut u8, usize, c_uint)>) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, buflen)?;
	do_getrandom(buf, flags)
}
