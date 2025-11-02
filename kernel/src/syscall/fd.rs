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
		FileType,
		fd::{NewFDConstraint, fd_to_file},
	},
	memory::user::{UserIOVec, UserPtr, UserSlice},
	process::Process,
};
use core::{
	cmp::min,
	ffi::{c_int, c_uint},
	hint::unlikely,
	sync::atomic::Ordering::{Acquire, Release},
};
use utils::{errno, errno::EResult, limits::IOV_MAX};

/// Sets the offset from the given value.
const SEEK_SET: u32 = 0;
/// Sets the offset relative to the current offset.
const SEEK_CUR: u32 = 1;
/// Sets the offset relative to the end of the file.
const SEEK_END: u32 = 2;

pub fn read(fd: c_int, buf: *mut u8, count: usize) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, count)?;
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let file = fd_to_file(fd)?;
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Read
	let off = file.off.load(Acquire);
	let len = file.ops.read(&file, off, buf)?;
	// Update offset
	let new_off = off.saturating_add(len as u64);
	file.off.store(new_off, Release);
	Ok(len as _)
}

pub fn pread64(fd: c_int, buf: *mut u8, count: usize, offset: u64) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, count)?;
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let file = fd_to_file(fd)?;
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	let len = file.ops.read(&file, offset, buf)?;
	Ok(len as _)
}

pub fn compat_pread64(
	fd: c_int,
	buf: *mut u8,
	count: usize,
	offset_low: u32,
	offset_high: u32,
) -> EResult<usize> {
	let offset = ((offset_high as u64) << 32) | (offset_low as u64);
	pread64(fd, buf, count, offset)
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
fn do_readv(
	fd: c_int,
	iov: UserIOVec,
	iovcnt: c_int,
	offset: Option<isize>,
	_flags: Option<i32>,
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
	let file = fd_to_file(fd)?;
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
			let off = file.off.load(Acquire);
			let len = file.ops.read(&file, off, buf)?;
			// Update offset
			let new_off = off.saturating_add(len as u64);
			file.off.store(new_off, Release);
			len
		};
		off += len;
		if unlikely(len < max_len) {
			break;
		}
	}
	Ok(off)
}

pub fn readv(fd: c_int, iov: UserIOVec, iovcnt: c_int) -> EResult<usize> {
	do_readv(fd, iov, iovcnt, None, None)
}

pub fn preadv(
	fd: c_int,
	iov: UserIOVec,
	iovcnt: c_int,
	offset_low: isize,
	offset_high: isize,
) -> EResult<usize> {
	#[allow(arithmetic_overflow)]
	let offset = offset_low | (offset_high << 32);
	do_readv(fd, iov, iovcnt, Some(offset), None)
}

pub fn preadv2(
	fd: c_int,
	iov: UserIOVec,
	iovcnt: c_int,
	offset: isize,
	flags: c_int,
) -> EResult<usize> {
	do_readv(fd, iov, iovcnt, Some(offset), Some(flags))
}

pub fn write(fd: c_int, buf: *mut u8, count: usize) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, count)?;
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let file = fd_to_file(fd)?;
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Write
	let off = file.off.load(Acquire);
	let len = file.ops.write(&file, off, buf)?;
	// Update offset
	let new_off = off.saturating_add(len as u64);
	file.off.store(new_off, Release);
	Ok(len)
}

pub fn pwrite64(fd: c_int, buf: *mut u8, count: usize, offset: u64) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, count)?;
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let file = fd_to_file(fd)?;
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	let len = file.ops.write(&file, offset, buf)?;
	Ok(len)
}

pub fn compat_pwrite64(
	fd: c_int,
	buf: *mut u8,
	count: usize,
	offset_low: u32,
	offset_high: u32,
) -> EResult<usize> {
	let offset = ((offset_high as u64) << 32) | (offset_low as u64);
	pwrite64(fd, buf, count, offset)
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
fn do_writev(
	fd: i32,
	iov: UserIOVec,
	iovcnt: i32,
	offset: Option<isize>,
	_flags: Option<i32>,
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
	let file = fd_to_file(fd)?;
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
			let off = file.off.load(Acquire);
			let len = file.ops.write(&file, off, buf)?;
			// Update offset
			let new_off = off.saturating_add(len as u64);
			file.off.store(new_off, Release);
			len
		};
		off += len;
	}
	Ok(off)
}

pub fn writev(fd: c_int, iov: UserIOVec, iovcnt: c_int) -> EResult<usize> {
	do_writev(fd, iov, iovcnt, None, None)
}

pub fn pwritev(
	fd: c_int,
	iov: UserIOVec,
	iovcnt: c_int,
	offset_low: isize,
	offset_high: isize,
) -> EResult<usize> {
	#[allow(arithmetic_overflow)]
	let offset = offset_low | (offset_high << 32);
	do_writev(fd, iov, iovcnt, Some(offset), None)
}

pub fn pwritev2(
	fd: c_int,
	iov: UserIOVec,
	iovcnt: c_int,
	offset: isize,
	flags: c_int,
) -> EResult<usize> {
	do_writev(fd, iov, iovcnt, Some(offset), Some(flags))
}

fn do_lseek(
	fd: c_uint,
	offset: i64,
	result: Option<UserPtr<u64>>,
	whence: c_uint,
) -> EResult<usize> {
	let file = fd_to_file(fd as _)?;
	// Compute the offset
	let base = match whence {
		SEEK_SET => 0,
		SEEK_CUR => file.off.load(Acquire),
		SEEK_END => file.stat().size,
		_ => return Err(errno!(EINVAL)),
	};
	let offset = match offset {
		// Positive offset
		0.. => base
			.checked_add(offset as _)
			.ok_or_else(|| errno!(EOVERFLOW))?,
		// Negative offset
		..0 => {
			let offset = offset.checked_abs().ok_or_else(|| errno!(EOVERFLOW))?;
			base.checked_sub(offset as _)
				.ok_or_else(|| errno!(EOVERFLOW))?
		}
	};
	if let Some(result) = result {
		// Write the result to the userspace
		result.copy_to_user(&offset)?;
	}
	// Set the new offset
	file.off.store(offset, Release);
	Ok(offset as _)
}

pub fn _llseek(
	fd: c_uint,
	offset_high: i32,
	offset_low: i32,
	result: UserPtr<u64>,
	whence: c_uint,
) -> EResult<usize> {
	let offset = ((offset_high as i64) << 32) | (offset_low as i64);
	do_lseek(fd, offset, Some(result), whence)?;
	Ok(0)
}

pub fn lseek(fd: c_uint, offset: i64, whence: c_uint) -> EResult<usize> {
	do_lseek(fd, offset, None, whence)
}

pub fn dup(oldfd: c_int) -> EResult<usize> {
	let (newfd_id, _) = Process::current().file_descriptors().lock().duplicate_fd(
		oldfd as _,
		NewFDConstraint::None,
		false,
	)?;
	Ok(newfd_id as _)
}

pub fn dup2(oldfd: c_int, newfd: c_int) -> EResult<usize> {
	let (newfd_id, _) = Process::current().file_descriptors().lock().duplicate_fd(
		oldfd as _,
		NewFDConstraint::Fixed(newfd as _),
		false,
	)?;
	Ok(newfd_id as _)
}

pub fn close(fd: c_int) -> EResult<usize> {
	Process::current()
		.file_descriptors()
		.lock()
		.close_fd(fd as _)?;
	Ok(0)
}
