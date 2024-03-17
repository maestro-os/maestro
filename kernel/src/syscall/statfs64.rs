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

//! The `statfs64` system call returns information about a mounted file system.

use super::statfs::do_statfs;
use crate::{
	file::fs::Statfs,
	process::mem_space::ptr::{SyscallPtr, SyscallString},
};
use macros::syscall;
use utils::errno::Errno;

// TODO Check args types
#[syscall]
pub fn statfs64(path: SyscallString, _sz: usize, buf: SyscallPtr<Statfs>) -> Result<i32, Errno> {
	// TODO Use `sz`
	do_statfs(path, buf)
}
