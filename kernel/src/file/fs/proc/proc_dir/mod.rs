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
	memory::{vmem, VirtAddr},
	process::mem_space::{copy::SyscallSlice, MemSpace},
	syscall::FromSyscallArg,
};
use core::{cmp::min, fmt, intrinsics::unlikely};
use utils::DisplayableStr;

pub mod cmdline;
pub mod cwd;
pub mod environ;
pub mod exe;
pub mod mounts;
pub mod stat;
pub mod status;

/// Reads a range of memory from `mem_space` and writes it to `f`.
///
/// `begin` and `end` represent the range of memory to read.
pub fn read_memory(
	f: &mut fmt::Formatter<'_>,
	mem_space: &MemSpace,
	begin: VirtAddr,
	end: VirtAddr,
) -> fmt::Result {
	if begin.is_null() {
		return Ok(());
	}
	let f = || {
		let slice = SyscallSlice::from_ptr(begin.0);
		let len = end.0.saturating_sub(begin.0);
		let mut i = 0;
		let mut buf: [u8; 128] = [0; 128];
		while i < len {
			let l = min(len - i, buf.len());
			let res = slice.copy_from_user(i, &mut buf[..l]);
			if unlikely(res.is_err()) {
				break;
			}
			write!(f, "{}", DisplayableStr(&buf[..l]))?;
			i += l;
		}
		Ok(())
	};
	unsafe { vmem::switch(&mem_space.vmem, f) }
}
