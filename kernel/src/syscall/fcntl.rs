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

//! The `fcntl` syscall call allows to manipulate a file descriptor.

use crate::{
	file::{buffer, buffer::pipe::PipeBuffer, fd::NewFDConstraint, FileType},
	process::Process,
};
use core::ffi::{c_int, c_void};
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// Duplicate the file descriptor using the lowest numbered available file descriptor greater than
/// or equal to the specified argument.
const F_DUPFD: i32 = 0;
/// Return the file descriptor flags.
const F_GETFD: i32 = 1;
/// Set the file descriptor flags.
const F_SETFD: i32 = 2;
/// Return the file access mode and the file status flags.
const F_GETFL: i32 = 3;
/// Set the file status flag.
const F_SETFL: i32 = 4;
/// TODO doc
const F_GETLK: i32 = 5;
/// TODO doc
const F_SETLK: i32 = 6;
/// TODO doc
const F_SETLKW: i32 = 7;
/// Set the process ID or process group ID that will receive `SIGIO` and `SIGURG` signals for
/// events on the file descriptor.
const F_SETOWN: i32 = 8;
/// Return the process ID or process group ID currently receiving `SIGIO` and `SIGURG` signals for
/// events on the file descriptor.
const F_GETOWN: i32 = 9;
/// Set the signal sent when input or output becomes possible to the given value.
const F_SETSIG: i32 = 10;
/// Return the signal sent when input or output becomes possible.
const F_GETSIG: i32 = 11;
/// TODO doc
const F_GETLK64: i32 = 12;
/// TODO doc
const F_SETLK64: i32 = 13;
/// TODO doc
const F_SETLKW64: i32 = 14;
/// Similar to `F_SETOWN`, except it allows to specifiy a thread ID using the `f_owner_ex`
/// structure.
const F_SETOWN_EX: i32 = 15;
/// Return the setting defined by `F_SETOWN_EX`.
const F_GETOWN_EX: i32 = 16;
/// TODO doc
const F_OFD_GETLK: i32 = 36;
/// TODO doc
const F_OFD_SETLK: i32 = 37;
/// TODO doc
const F_OFD_SETLKW: i32 = 38;
/// Set or remove a file lease.
const F_SETLEASE: i32 = 1024;
/// Indicates what type of lease is associated with the file descriptor.
const F_GETLEASE: i32 = 1025;
/// Provide notification when the directory referred to by the file descriptor or any of the files
/// that it contains is changed.
const F_NOTIFY: i32 = 1026;
/// Like `F_DUPFD`, but also set the close-on-exec flag for the duplicate file descritpr.
const F_DUPFD_CLOEXEC: i32 = 1030;
/// Change the capacity of the pipe referred to by the file descriptor to be at least the given
/// number of bytes.
const F_SETPIPE_SZ: i32 = 1031;
/// Return the capacity of the pipe referred to bt the file descriptor.
const F_GETPIPE_SZ: i32 = 1032;
/// Add the seals given in the bit-mask argument to the set of seeals of the inode referred to by
/// the file descriptor.
const F_ADD_SEALS: i32 = 1033;
/// Return the current set of seals of the inode referred to by the file descriptor.
const F_GET_SEALS: i32 = 1034;
/// Return the value of the read/write hint associated with the underlying inode referred to by the
/// file descriptor.
const F_GET_RW_HINT: i32 = 1035;
/// Set the read/write hint value associated with the underlying inode referred to by the file
/// descriptor.
const F_SET_RW_HINT: i32 = 1036;
/// Return the value of the read/write hint associated with the open file description referred to
/// by the file descriptor.
const F_GET_FILE_RW_HINT: i32 = 1037;
/// Set the read/write hint value associated with the open file description referred to by the file
/// descriptor.
const F_SET_FILE_RW_HINT: i32 = 1038;

/// TODO doc
const F_SEAL_FUTURE_WRITE: i32 = 16;

/// Take out a read lease.
const F_RDLCK: i32 = 0;
/// Take out a write lease.
const F_WRLCK: i32 = 1;
/// Remove our lease from the file.
const F_UNLCK: i32 = 2;

/// Send the signal to the process group whose ID is specified.
const F_OWNER_PGRP: i32 = 2;
/// Send the signal to the process whose ID is specified.
const F_OWNER_PID: i32 = 1;
/// Send the signal to the thread whose thread ID is specified.
const F_OWNER_TID: i32 = 0;

/// If this seal is set, the size of the file in question cannot be increased.
const F_SEAL_GROW: i32 = 4;
/// If this seal is set, any further call to `fcntl` with `F_ADD_SEALS` fails.
const F_SEAL_SEAL: i32 = 1;
/// If this seal is set, the size of the file in question cannot be reduced.
const F_SEAL_SHRINK: i32 = 2;
/// If this seal is set, you cannot modify the contents of the file.
const F_SEAL_WRITE: i32 = 8;

/// Performs the fcntl system call.
///
/// `fcntl64` tells whether this is the `fcntl64` system call.
pub fn do_fcntl(fd: i32, cmd: i32, arg: *mut c_void, _fcntl64: bool) -> EResult<i32> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let fds_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		proc.file_descriptors.clone().unwrap()
	};
	let mut fds = fds_mutex.lock();

	match cmd {
		F_DUPFD => Ok(fds
			.duplicate_fd(fd as _, NewFDConstraint::Min(arg as _), false)?
			.get_id() as _),
		F_GETFD => {
			let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
			Ok(fd.get_flags())
		}
		F_SETFD => {
			let fd = fds.get_fd_mut(fd as _).ok_or_else(|| errno!(EBADF))?;
			fd.set_flags(arg as _);
			Ok(0)
		}
		F_GETFL => {
			let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
			let open_file_mutex = fd.get_open_file();
			let open_file = open_file_mutex.lock();
			Ok(open_file.get_flags())
		}
		F_SETFL => {
			let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
			let open_file_mutex = fd.get_open_file();
			let mut open_file = open_file_mutex.lock();
			open_file.set_flags(arg as _);
			Ok(0)
		}
		F_GETLK => {
			// TODO
			todo!();
		}
		F_SETLK => {
			// TODO
			todo!();
		}
		F_SETLKW => {
			// TODO
			todo!();
		}
		F_SETOWN => {
			// TODO
			todo!();
		}
		F_GETOWN => {
			// TODO
			todo!();
		}
		F_SETSIG => {
			// TODO
			todo!();
		}
		F_GETSIG => {
			// TODO
			todo!();
		}
		F_GETLK64 => {
			// TODO
			todo!();
		}
		F_SETLK64 => {
			// TODO
			todo!();
		}
		F_SETLKW64 => {
			// TODO
			todo!();
		}
		F_SETOWN_EX => {
			// TODO
			todo!();
		}
		F_GETOWN_EX => {
			// TODO
			todo!();
		}
		F_OFD_GETLK => {
			// TODO
			todo!();
		}
		F_OFD_SETLK => {
			// TODO
			todo!();
		}
		F_OFD_SETLKW => {
			// TODO
			todo!();
		}
		F_SETLEASE => {
			// TODO
			todo!();
		}
		F_GETLEASE => {
			// TODO
			todo!();
		}
		F_NOTIFY => {
			// TODO
			todo!();
		}
		F_DUPFD_CLOEXEC => Ok(fds
			.duplicate_fd(fd as _, NewFDConstraint::Min(arg as _), true)?
			.get_id() as _),
		F_SETPIPE_SZ => {
			// TODO
			todo!();
		}
		F_GETPIPE_SZ => {
			let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
			let open_file_mutex = fd.get_open_file();
			let open_file = open_file_mutex.lock();
			let file_mutex = open_file.get_file();
			let file = file_mutex.lock();
			match file.get_content() {
				FileType::Fifo => Ok(buffer::get_or_default::<PipeBuffer>(file.get_location())?
					.lock()
					.get_capacity() as _),
				_ => Ok(0),
			}
		}
		F_ADD_SEALS => {
			// TODO
			todo!();
		}
		F_GET_SEALS => {
			// TODO
			todo!();
		}
		F_GET_RW_HINT => {
			// TODO
			todo!();
		}
		F_SET_RW_HINT => {
			// TODO
			todo!();
		}
		F_GET_FILE_RW_HINT => {
			// TODO
			todo!();
		}
		F_SET_FILE_RW_HINT => {
			// TODO
			todo!();
		}
		_ => Err(errno!(EINVAL)),
	}
}

#[syscall]
pub fn fcntl(fd: c_int, cmd: c_int, arg: *mut c_void) -> EResult<i32> {
	do_fcntl(fd, cmd, arg, false)
}
