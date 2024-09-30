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
	file::{fd::FileDescriptorTable, FileType, INode},
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use core::{
	ffi::c_uint,
	mem::{offset_of, size_of},
	ops::Range,
	ptr,
	sync::atomic,
};
use utils::{
	bytes::as_bytes,
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
	vec,
};

/// A directory entry as returned by the `getdents*` system calls.
pub trait Dirent: Sized {
	/// The maximum value fitting in the structure for the inode.
	const INODE_MAX: u64;

	/// Returns the number of bytes required for an entry with the given name.
	///
	/// This function must return a number that ensures the entry is aligned in memory (a multiple
	/// of `4` or `8` depending on the architecture).
	fn required_length(name: &[u8]) -> usize;

	/// Writes a new entry on the given slice.
	///
	/// Arguments:
	/// - `slice` is the slice to write on.
	/// - `off` is the offset at which the entry is to be written.
	/// - `inode` is the inode of the entry.
	/// - `entry_type` is the type of the entry.
	/// - `name` is the name of the entry.
	fn write(
		slice: &SyscallSlice<u8>,
		off: usize,
		inode: INode,
		entry_type: FileType,
		name: &[u8],
	) -> EResult<()>;
}

/// Performs the `getdents` system call.
pub fn do_getdents<E: Dirent>(
	fd: c_uint,
	dirp: SyscallSlice<u8>,
	count: usize,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let file = fds.lock().get_fd(fd as _)?.get_file().clone();
	let node = file
		.vfs_entry
		.as_ref()
		.ok_or_else(|| errno!(ENOTDIR))?
		.node();
	let mut off = file.off.load(atomic::Ordering::Acquire);
	let mut buf_off = 0;
	// Iterate over entries and fill the buffer
	loop {
		let Some((entry, next_off)) = node.ops.next_entry(&node.location, off)? else {
			break;
		};
		// Skip entries whose inode cannot fit in the structure
		if entry.inode > E::INODE_MAX {
			continue;
		}
		let len = E::required_length(entry.name.as_ref());
		// If the buffer is not large enough, return an error
		if buf_off == 0 && len > count {
			return Err(errno!(EINVAL));
		}
		// If reaching the end of the buffer, break
		if buf_off + len > count {
			break;
		}
		E::write(
			&dirp,
			buf_off,
			entry.inode,
			entry.entry_type,
			entry.name.as_ref(),
		)?;
		buf_off += len;
		off = next_off;
	}
	file.off.store(off, atomic::Ordering::Release);
	Ok(buf_off as _)
}

/// A Linux directory entry.
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
	const INODE_MAX: u64 = u32::MAX as _;

	fn required_length(name: &[u8]) -> usize {
		(size_of::<Self>() + name.len() + 2)
			// Padding for alignment
			.next_multiple_of(size_of::<usize>())
	}

	fn write(
		slice: &SyscallSlice<u8>,
		off: usize,
		inode: INode,
		entry_type: FileType,
		name: &[u8],
	) -> EResult<()> {
		let len = Self::required_length(name);
		let ent = Self {
			d_ino: inode as _,
			d_off: (off + len) as _,
			d_reclen: len as _,
			d_name: [],
		};
		// Write entry
		slice.copy_to_user(off, as_bytes(&ent))?;
		// Copy file name
		slice.copy_to_user(off + offset_of!(Self, d_name), name)?;
		// Write nul byte and entry type
		slice.copy_to_user(
			off + offset_of!(Self, d_name) + name.len(),
			&[b'\0', entry_type.to_dirent_type()],
		)?;
		Ok(())
	}
}

pub fn getdents(
	Args((fd, dirp, count)): Args<(c_uint, SyscallSlice<u8>, c_uint)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_getdents::<LinuxDirent>(fd, dirp, count as usize, fds)
}
