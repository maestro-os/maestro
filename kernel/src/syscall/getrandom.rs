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
	process::{mem_space::ptr::SyscallSlice, Process},
};
use core::ffi::c_uint;
use macros::syscall;
use utils::{errno, errno::Errno};

/// If set, bytes are draw from the random source instead of urandom.
const GRND_RANDOM: u32 = 2;
/// If set, the function doesn't block. If no entropy is available, the function
/// returns EAGAIN.
const GRND_NONBLOCK: u32 = 1;

#[syscall]
pub fn getrandom(buf: SyscallSlice<u8>, buflen: usize, flags: c_uint) -> Result<i32, Errno> {
	let bypass_threshold = flags & GRND_RANDOM == 0;
	let nonblock = flags & GRND_NONBLOCK != 0;

	let mut pool_guard = rand::ENTROPY_POOL.lock();
	let Some(pool) = &mut *pool_guard else {
		return Ok(0);
	};

	if nonblock && buflen > pool.available_bytes() {
		return Err(errno!(EAGAIN));
	}

	// Getting current process
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space_mutex = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space_mutex.lock();

	if let Some(buf) = buf.get_mut(&mut mem_space_guard, buflen)? {
		let mut i = 0;
		while i < buf.len() {
			i += pool.read(&mut buf[i..], bypass_threshold);
		}

		Ok(buf.len() as _)
	} else {
		Ok(0)
	}
}
