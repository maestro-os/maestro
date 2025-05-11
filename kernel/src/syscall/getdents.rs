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
	file::{DT_UNKNOWN, DirContext, DirEntry, FileType, INode, fd::FileDescriptorTable},
	memory::user::UserSlice,
	process::Process,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{
	ffi::{c_int, c_uint},
	mem::{offset_of, size_of},
	ops::Range,
	ptr,
	sync::atomic,
};
use utils::{
	bytes::as_bytes,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
	vec,
};

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

/// A Linux directory entry with 64 bits offsets.
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

fn do_getdents<F: FnMut(&DirEntry) -> EResult<bool>>(
	fd: c_int,
	fds: Arc<Mutex<FileDescriptorTable>>,
	mut write: F,
) -> EResult<()> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}
	let file = fds.lock().get_fd(fd as _)?.get_file().clone();
	if file.stat()?.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	let mut ctx = DirContext {
		write: &mut write,
		off: file.off.load(atomic::Ordering::Acquire),
	};
	// cannot fail since we know this is a directory
	let node = file.node().unwrap();
	node.node_ops.iter_entries(node, &mut ctx)?;
	file.off.store(ctx.off, atomic::Ordering::Release);
	Ok(())
}

pub fn getdents(
	Args((fd, dirp, count)): Args<(c_int, *mut u8, c_uint)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let count = count as usize;
	let dirp = UserSlice::from_user(dirp, count)?;
	let mut buf_off = 0;
	do_getdents(fd, fds, |entry| {
		// Skip entries whose inode cannot fit in the structure
		if entry.inode > u32::MAX as _ {
			return Ok(true);
		}
		let reclen = (size_of::<LinuxDirent>() + entry.name.len() + 2)
			// Padding for alignment
			.next_multiple_of(size_of::<usize>());
		// If the buffer is not large enough, return an error
		if buf_off == 0 && reclen > count {
			return Err(errno!(EINVAL));
		}
		// If reaching the end of the buffer, stop
		if buf_off + reclen > count {
			return Ok(false);
		}
		let d_type = entry
			.entry_type
			.map(FileType::to_dirent_type)
			.unwrap_or(DT_UNKNOWN);
		// Write entry
		let ent = LinuxDirent {
			d_ino: entry.inode as _,
			d_off: (buf_off + reclen) as _,
			d_reclen: reclen as _,
			d_name: [],
		};
		// Write entry
		dirp.copy_to_user(buf_off, as_bytes(&ent))?;
		// Copy file name
		dirp.copy_to_user(buf_off + offset_of!(LinuxDirent, d_name), entry.name)?;
		// Write nul byte and entry type
		dirp.copy_to_user(
			buf_off + offset_of!(LinuxDirent, d_name) + entry.name.len(),
			&[b'\0', d_type],
		)?;
		buf_off += reclen;
		Ok(true)
	})?;
	Ok(buf_off)
}

pub fn getdents64(
	Args((fd, dirp, count)): Args<(c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let dirp = UserSlice::from_user(dirp, count)?;
	let mut buf_off = 0;
	do_getdents(fd as _, fds, |entry| {
		let reclen = (size_of::<LinuxDirent64>() + entry.name.len() + 1)
			// Padding for alignment
			.next_multiple_of(align_of::<LinuxDirent64>());
		// If the buffer is not large enough, return an error
		if buf_off == 0 && reclen > count {
			return Err(errno!(EINVAL));
		}
		// If reaching the end of the buffer, stop
		if buf_off + reclen > count {
			return Ok(false);
		}
		let d_type = entry
			.entry_type
			.map(FileType::to_dirent_type)
			.unwrap_or(DT_UNKNOWN);
		// Write entry
		let ent = LinuxDirent64 {
			d_ino: entry.inode,
			d_off: (buf_off + reclen) as _,
			d_reclen: reclen as _,
			d_type,
			d_name: [],
		};
		dirp.copy_to_user(buf_off, as_bytes(&ent))?;
		// Copy file name
		dirp.copy_to_user(buf_off + offset_of!(LinuxDirent64, d_name), entry.name)?;
		dirp.copy_to_user(
			buf_off + offset_of!(LinuxDirent64, d_name) + entry.name.len(),
			b"\0",
		)?;
		buf_off += reclen;
		Ok(true)
	})?;
	Ok(buf_off)
}
