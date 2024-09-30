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
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use core::ffi::c_uint;
use utils::{
	errno,
	errno::{EResult, Errno},
	vec,
};

/// If set, bytes are drawn from the randomness source instead of `urandom`.
const GRND_RANDOM: u32 = 2;
/// If set, the function doesn't block. If no entropy is available, the function
/// returns [`EAGAIN`].
const GRND_NONBLOCK: u32 = 1;

pub fn getrandom(
	Args((buf, buflen, flags)): Args<(SyscallSlice<u8>, usize, c_uint)>,
) -> EResult<usize> {
	let bypass_threshold = flags & GRND_RANDOM == 0;
	let nonblock = flags & GRND_NONBLOCK != 0;
	let mut pool_guard = rand::ENTROPY_POOL.lock();
	let Some(pool) = &mut *pool_guard else {
		return Ok(0);
	};
	if nonblock && buflen > pool.available_bytes() {
		return Err(errno!(EAGAIN));
	}
	// Write
	let mut tmp: [u8; 256] = [0; 256];
	let mut i = 0;
	while i < buflen {
		let len = pool.read(&mut tmp[..buflen - i], bypass_threshold);
		buf.copy_to_user(i, &tmp[..len])?;
		i += len;
	}
	Ok(i as _)
}
