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

//! Implementation of the directory of a process in the proc.

use crate::{
	memory::{VirtAddr, user::UserSlice},
	process::mem_space::MemSpace,
};
use utils::{collections::vec::Vec, errno::AllocResult, ptr::arc::Arc, vec};

pub mod cmdline;
pub mod cwd;
pub mod environ;
pub mod exe;
pub mod maps;
pub mod mounts;
pub mod stat;
pub mod status;

/// Reads a range of memory from `mem_space` and writes it to `f`.
///
/// `begin` and `end` represent the range of memory to read.
pub fn read_memory(
	mem_space: &Arc<MemSpace>,
	begin: VirtAddr,
	end: VirtAddr,
) -> AllocResult<Vec<u8>> {
	let len = end.0.saturating_sub(begin.0);
	let mut buf = vec![0; len]?;
	let Ok(slice) = UserSlice::from_user(begin.as_ptr(), len) else {
		// Slice is out of range: return zeros
		return Ok(buf);
	};
	MemSpace::switch(mem_space, |_| {
		let mut i = 0;
		while i < len {
			let Ok(len) = slice.copy_from_user(i, &mut buf[i..]) else {
				break;
			};
			i += len;
		}
	});
	Ok(buf)
}
