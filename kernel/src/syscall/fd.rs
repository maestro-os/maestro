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

//! File descriptors handling system calls.

use crate::{
	file::{
		File, FileType,
		fd::{FileDescriptorTable, NewFDConstraint},
	},
	memory::user::{UserIOVec, UserPtr, UserSlice},
	process::scheduler::Scheduler,
	sync::mutex::Mutex,
	syscall::Args,
	time::{
		clock::{Clock, current_time_ms},
		unit::Timestamp,
	},
};
use core::{
	cmp::min,
	ffi::{c_int, c_uint, c_ulong, c_void},
	intrinsics::unlikely,
	sync::atomic,
};
use utils::{errno, errno::EResult, limits::IOV_MAX, ptr::arc::Arc};

/// Sets the offset from the given value.
const SEEK_SET: u32 = 0;
/// Sets the offset relative to the current offset.
const SEEK_CUR: u32 = 1;
/// Sets the offset relative to the end of the file.
const SEEK_END: u32 = 2;

pub fn read(
	Args((fd, buf, count)): Args<(c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, count)?;
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Read
	let off = file.off.load(atomic::Ordering::Acquire);
	let len = file.ops.read(&file, off, buf)?;
	// Update offset
	let new_off = off.saturating_add(len as u64);
	file.off.store(new_off, atomic::Ordering::Release);
	Ok(len as _)
}

// FIXME: the operation has to be atomic
/// Performs the readv operation.
///
/// Arguments:
/// - `fd` is the file descriptor
/// - `iov` the IO vector
/// - `iovcnt` the number of entries in the IO vector
/// - `offset` is the offset in the file
/// - `flags` is the set of flags
pub fn do_readv(
	fd: c_int,
	iov: UserIOVec,
	iovcnt: c_int,
	offset: Option<isize>,
	_flags: Option<i32>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if unlikely(iovcnt < 0 || iovcnt as usize > IOV_MAX) {
		return Err(errno!(EINVAL));
	}
	let offset = match offset {
		Some(o @ 0..) => Some(o as u64),
		None | Some(-1) => None,
		Some(..-1) => return Err(errno!(EINVAL)),
	};
	// TODO Handle flags
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Read
	let mut off = 0;
	for i in iov.iter(iovcnt as _) {
		let i = i?;
		// The size to read. This is limited to avoid an overflow on the total length
		let max_len = min(i.iov_len, i32::MAX as usize - off);
		let buf = UserSlice::<u8>::from_user(i.iov_base, max_len)?;
		// Read
		let len = if let Some(offset) = offset {
			let file_off = offset + off as u64;
			file.ops.read(&file, file_off, buf)?
		} else {
			let off = file.off.load(atomic::Ordering::Acquire);
			let len = file.ops.read(&file, off, buf)?;
			// Update offset
			let new_off = off.saturating_add(len as u64);
			file.off.store(new_off, atomic::Ordering::Release);
			len
		};
		off += len;
		if unlikely(len < max_len) {
			break;
		}
	}
	Ok(off)
}

pub fn readv(
	Args((fd, iov, iovcnt)): Args<(c_int, UserIOVec, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_readv(fd, iov, iovcnt, None, None, fds)
}

pub fn preadv(
	Args((fd, iov, iovcnt, offset_low, offset_high)): Args<(
		c_int,
		UserIOVec,
		c_int,
		isize,
		isize,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	#[allow(arithmetic_overflow)]
	let offset = offset_low | (offset_high << 32);
	do_readv(fd, iov, iovcnt, Some(offset), None, fds)
}

pub fn preadv2(
	Args((fd, iov, iovcnt, offset, flags)): Args<(c_int, UserIOVec, c_int, isize, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_readv(fd, iov, iovcnt, Some(offset), Some(flags), fds)
}

pub fn write(
	Args((fd, buf, count)): Args<(c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, count)?;
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Write
	let off = file.off.load(atomic::Ordering::Acquire);
	let len = file.ops.write(&file, off, buf)?;
	// Update offset
	let new_off = off.saturating_add(len as u64);
	file.off.store(new_off, atomic::Ordering::Release);
	Ok(len)
}

// FIXME: the operation has to be atomic
/// Performs the `writev` operation.
///
/// Arguments:
/// - `fd` is the file descriptor
/// - `iov` the IO vector
/// - `iovcnt` the number of entries in the IO vector
/// - `offset` is the offset in the file
/// - `flags` is the set of flags
pub fn do_writev(
	fd: i32,
	iov: UserIOVec,
	iovcnt: i32,
	offset: Option<isize>,
	_flags: Option<i32>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if iovcnt < 0 || iovcnt as usize > IOV_MAX {
		return Err(errno!(EINVAL));
	}
	let offset = match offset {
		Some(o @ 0..) => Some(o as u64),
		None | Some(-1) => None,
		Some(..-1) => return Err(errno!(EINVAL)),
	};
	// Get file
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Write
	let mut off = 0;
	for i in iov.iter(iovcnt as _) {
		let i = i?;
		// The size to write. This is limited to avoid an overflow on the total length
		let len = min(i.iov_len, i32::MAX as usize - off);
		let buf = UserSlice::<u8>::from_user(i.iov_base, len)?;
		let len = if let Some(offset) = offset {
			let file_off = offset + off as u64;
			file.ops.write(&file, file_off, buf)?
		} else {
			let off = file.off.load(atomic::Ordering::Acquire);
			let len = file.ops.write(&file, off, buf)?;
			// Update offset
			let new_off = off.saturating_add(len as u64);
			file.off.store(new_off, atomic::Ordering::Release);
			len
		};
		off += len;
	}
	Ok(off)
}

pub fn writev(
	Args((fd, iov, iovcnt)): Args<(c_int, UserIOVec, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_writev(fd, iov, iovcnt, None, None, fds)
}

pub fn pwritev(
	Args((fd, iov, iovcnt, offset_low, offset_high)): Args<(
		c_int,
		UserIOVec,
		c_int,
		isize,
		isize,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	#[allow(arithmetic_overflow)]
	let offset = offset_low | (offset_high << 32);
	do_writev(fd, iov, iovcnt, Some(offset), None, fds)
}

pub fn pwritev2(
	Args((fd, iov, iovcnt, offset, flags)): Args<(c_int, UserIOVec, c_int, isize, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_writev(fd, iov, iovcnt, Some(offset), Some(flags), fds)
}

fn do_lseek(
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	fd: c_uint,
	offset: u64,
	result: Option<UserPtr<u64>>,
	whence: c_uint,
) -> EResult<usize> {
	let fds = fds_mutex.lock();
	let file = fds.get_fd(fd as _)?.get_file();
	// Compute the offset
	let base = match whence {
		SEEK_SET => 0,
		SEEK_CUR => file.off.load(atomic::Ordering::Acquire),
		SEEK_END => file.stat()?.size,
		_ => return Err(errno!(EINVAL)),
	};
	let offset = base.checked_add(offset).ok_or_else(|| errno!(EOVERFLOW))?;
	if let Some(result) = result {
		// Write the result to the userspace
		result.copy_to_user(&offset)?;
	}
	// Set the new offset
	file.off.store(offset, atomic::Ordering::Release);
	Ok(offset as _)
}

pub fn _llseek(
	Args((fd, offset_high, offset_low, result, whence)): Args<(
		c_uint,
		u32,
		u32,
		UserPtr<u64>,
		c_uint,
	)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let offset = ((offset_high as u64) << 32) | (offset_low as u64);
	do_lseek(fds_mutex, fd, offset, Some(result), whence)?;
	Ok(0)
}

pub fn lseek(
	Args((fd, offset, whence)): Args<(c_uint, u64, c_uint)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_lseek(fds_mutex, fd, offset, None, whence)
}

pub fn dup(Args(oldfd): Args<c_int>, fds: Arc<Mutex<FileDescriptorTable>>) -> EResult<usize> {
	let (newfd_id, _) = fds
		.lock()
		.duplicate_fd(oldfd as _, NewFDConstraint::None, false)?;
	Ok(newfd_id as _)
}

pub fn dup2(
	Args((oldfd, newfd)): Args<(c_int, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let (newfd_id, _) =
		fds.lock()
			.duplicate_fd(oldfd as _, NewFDConstraint::Fixed(newfd as _), false)?;
	Ok(newfd_id as _)
}

pub fn close(Args(fd): Args<c_int>, fds: Arc<Mutex<FileDescriptorTable>>) -> EResult<usize> {
	fds.lock().close_fd(fd as _)?;
	Ok(0)
}
