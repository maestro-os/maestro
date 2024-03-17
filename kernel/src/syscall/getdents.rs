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

//! The `getdents` system call allows to get the list of entries in a given
//! directory.

use crate::{
	file::{FileContent, FileType, INode},
	process::{mem_space::ptr::SyscallSlice, Process},
};
use core::{
	ffi::c_uint,
	mem::{offset_of, size_of},
	ptr,
};
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// A directory entry as returned by the `getdents*` system calls.
pub trait Dirent: Sized {
	/// Returns the number of bytes required for an entry with the given name.
	///
	/// This function must return a number that ensures the entry is aligned in memory (a multiple
	/// of `4` or `8` depending on the architecture).
	fn required_length(name: &[u8]) -> usize;

	/// Writes a new entry on the given slice.
	///
	/// Arguments:
	/// - `slice` is the slice to write on.
	/// - `off` is the offset to the beginning of the entry in the slice.
	/// - `inode` is the inode of the entry.
	/// - `entry_type` is the type of the entry.
	/// - `name` is the name of the entry.
	fn write(slice: &mut [u8], off: usize, inode: INode, entry_type: FileType, name: &[u8]);
}

/// Performs the getdents system call.
pub fn do_getdents<E: Dirent>(fd: c_uint, dirp: SyscallSlice<u8>, count: usize) -> EResult<i32> {
	let (mem_space, open_file_mutex) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap().clone();

		let fds_mutex = proc.file_descriptors.clone().unwrap();
		let fds = fds_mutex.lock();

		let open_file_mutex = fds
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()
			.clone();

		(mem_space, open_file_mutex)
	};

	let mut mem_space_guard = mem_space.lock();
	let dirp_slice = dirp
		.get_mut(&mut mem_space_guard, count as _)?
		.ok_or_else(|| errno!(EFAULT))?;

	let mut open_file = open_file_mutex.lock();
	let start = open_file.get_offset();

	let mut off = 0;
	let mut entries_count = 0;

	{
		let file_mutex = open_file.get_file();
		let file = file_mutex.lock();

		let FileContent::Directory(entries) = file.get_content() else {
			return Err(errno!(ENOTDIR));
		};
		// TODO skip entries whose inode cannot fit in struct
		let entries = entries.iter().skip(start as _);

		// Iterate over entries and fill the buffer
		for (name, entry) in entries {
			let len = E::required_length(name);
			// If the buffer is not large enough, return an error
			if off == 0 && len > count {
				return Err(errno!(EINVAL));
			}
			// If reaching the end of the buffer, break
			if off + len > count {
				break;
			}

			E::write(dirp_slice, off, entry.inode, entry.entry_type, name);

			off += len;
			entries_count += 1;
		}
	}

	open_file.set_offset(start + entries_count);
	Ok(off as _)
}

/// Structure representing a Linux directory entry.
#[repr(C)]
struct LinuxDirent {
	/// Inode number.
	d_ino: u32,
	/// Offset to the next entry.
	d_off: u32,
	/// Length of this entry.
	d_reclen: u16,
	/// Filename (nul-terminated).
	///
	/// The filename is immediately followed by a zero padding byte, then a byte
	/// indicating the type of the entry.
	d_name: [u8; 0],
}

impl Dirent for LinuxDirent {
	fn required_length(name: &[u8]) -> usize {
		(size_of::<Self>() + name.len() + 2)
			// Padding for alignment
			.next_multiple_of(size_of::<usize>())
	}

	fn write(slice: &mut [u8], off: usize, inode: INode, entry_type: FileType, name: &[u8]) {
		let len = Self::required_length(name);
		let ent = Self {
			d_ino: inode as _,
			d_off: (off + len) as _,
			d_reclen: len as _,
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
		// Write entry type
		name_slice[name.len() + 1] = entry_type.to_dirent_type();
	}
}

#[syscall]
pub fn getdents(fd: c_uint, dirp: SyscallSlice<u8>, count: c_uint) -> Result<i32, Errno> {
	do_getdents::<LinuxDirent>(fd, dirp, count as usize)
}
