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

//! The `getdents64` system call allows to get the list of entries in a given
//! directory.

use super::getdents::{do_getdents, Dirent};
use crate::{
	file::{FileType, INode},
	process::mem_space::ptr::SyscallSlice,
};
use core::{
	ffi::c_int,
	mem::{offset_of, size_of},
	ptr,
};
use macros::syscall;
use utils::{errno, errno::Errno};

/// Structure representing a Linux directory entry with 64 bits offsets.
#[repr(C)]
struct LinuxDirent64 {
	/// 64-bit inode number.
	d_ino: u64,
	/// 64-bit offset to next entry.
	d_off: u64,
	/// Size of this dirent.
	d_reclen: u16,
	/// File type.
	d_type: u8,
	/// Filename (nul-terminated).
	d_name: [u8; 0],
}

impl Dirent for LinuxDirent64 {
	fn required_length(name: &[u8]) -> usize {
		(size_of::<Self>() + name.len() + 1)
			// Padding for alignment
			.next_multiple_of(size_of::<usize>())
	}

	fn write(slice: &mut [u8], off: usize, inode: INode, entry_type: FileType, name: &[u8]) {
		let len = Self::required_length(name);
		let ent = Self {
			d_ino: inode,
			d_off: (off + len) as _,
			d_reclen: len as _,
			d_type: entry_type.to_dirent_type(),
			d_name: [],
		};

		// Write entry
		unsafe {
			ptr::write(&mut slice[off] as *mut _ as *mut _, ent);
		}
		// Copy file name
		let name_slice = &mut slice[off + offset_of!(Self, d_name)..];
		name_slice[..name.len()].copy_from_slice(name);
		name_slice[name.len()] = 0;
	}
}

#[syscall]
pub fn getdents64(fd: c_int, dirp: SyscallSlice<u8>, count: usize) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}
	do_getdents::<LinuxDirent64>(fd as _, dirp, count)
}
