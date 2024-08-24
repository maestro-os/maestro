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
	file::{
		buffer,
		buffer::pipe::PipeBuffer,
		fd::{FileDescriptorTable, NewFDConstraint},
		FileType,
	},
	process::Process,
	syscall::Args,
};
use core::{
	any::Any,
	ffi::{c_int, c_void},
};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

/// Duplicate the file descriptor using the lowest numbered available file descriptor greater than
/// or equal to the specified argument.
const F_DUPFD: c_int = 0;
/// Return the file descriptor flags.
const F_GETFD: c_int = 1;
/// Set the file descriptor flags.
const F_SETFD: c_int = 2;
/// Return the file access mode and the file status flags.
const F_GETFL: c_int = 3;
/// Set the file status flag.
const F_SETFL: c_int = 4;
/// TODO doc
const F_GETLK: c_int = 5;
/// TODO doc
const F_SETLK: c_int = 6;
/// TODO doc
const F_SETLKW: c_int = 7;
/// Set the process ID or process group ID that will receive `SIGIO` and `SIGURG` signals for
/// events on the file descriptor.
const F_SETOWN: c_int = 8;
/// Return the process ID or process group ID currently receiving `SIGIO` and `SIGURG` signals for
/// events on the file descriptor.
const F_GETOWN: c_int = 9;
/// Set the signal sent when input or output becomes possible to the given value.
const F_SETSIG: c_int = 10;
/// Return the signal sent when input or output becomes possible.
const F_GETSIG: c_int = 11;
/// TODO doc
const F_GETLK64: c_int = 12;
/// TODO doc
const F_SETLK64: c_int = 13;
/// TODO doc
const F_SETLKW64: c_int = 14;
/// Similar to `F_SETOWN`, except it allows to specifiy a thread ID using the `f_owner_ex`
/// structure.
const F_SETOWN_EX: c_int = 15;
/// Return the setting defined by `F_SETOWN_EX`.
const F_GETOWN_EX: c_int = 16;
/// TODO doc
const F_OFD_GETLK: c_int = 36;
/// TODO doc
const F_OFD_SETLK: c_int = 37;
/// TODO doc
const F_OFD_SETLKW: c_int = 38;
/// Set or remove a file lease.
const F_SETLEASE: c_int = 1024;
/// Indicates what type of lease is associated with the file descriptor.
const F_GETLEASE: c_int = 1025;
/// Provide notification when the directory referred to by the file descriptor or any of the files
/// that it contains is changed.
const F_NOTIFY: c_int = 1026;
/// Like `F_DUPFD`, but also set the close-on-exec flag for the duplicate file descriptor.
const F_DUPFD_CLOEXEC: c_int = 1030;
/// Change the capacity of the pipe referred to by the file descriptor to be at least the given
/// number of bytes.
const F_SETPIPE_SZ: c_int = 1031;
/// Return the capacity of the pipe referred to bt the file descriptor.
const F_GETPIPE_SZ: c_int = 1032;
/// Add the seals given in the bit-mask argument to the set of seals of the inode referred to by
/// the file descriptor.
const F_ADD_SEALS: c_int = 1033;
/// Return the current set of seals of the inode referred to by the file descriptor.
const F_GET_SEALS: c_int = 1034;
/// Return the value of the read/write hint associated with the underlying inode referred to by the
/// file descriptor.
const F_GET_RW_HINT: c_int = 1035;
/// Set the read/write hint value associated with the underlying inode referred to by the file
/// descriptor.
const F_SET_RW_HINT: c_int = 1036;
/// Return the value of the read/write hint associated with the open file description referred to
/// by the file descriptor.
const F_GET_FILE_RW_HINT: c_int = 1037;
/// Set the read/write hint value associated with the open file description referred to by the file
/// descriptor.
const F_SET_FILE_RW_HINT: c_int = 1038;

/// TODO doc
const F_SEAL_FUTURE_WRITE: c_int = 16;

/// Take out a read lease.
const F_RDLCK: c_int = 0;
/// Take out a write lease.
const F_WRLCK: c_int = 1;
/// Remove our lease from the file.
const F_UNLCK: c_int = 2;

/// Send the signal to the process group whose ID is specified.
const F_OWNER_PGRP: c_int = 2;
/// Send the signal to the process whose ID is specified.
const F_OWNER_PID: c_int = 1;
/// Send the signal to the thread whose thread ID is specified.
const F_OWNER_TID: c_int = 0;

/// If this seal is set, the size of the file in question cannot be increased.
const F_SEAL_GROW: c_int = 4;
/// If this seal is set, any further call to `fcntl` with `F_ADD_SEALS` fails.
const F_SEAL_SEAL: c_int = 1;
/// If this seal is set, the size of the file in question cannot be reduced.
const F_SEAL_SHRINK: c_int = 2;
/// If this seal is set, you cannot modify the contents of the file.
const F_SEAL_WRITE: c_int = 8;

/// Performs the fcntl system call.
///
/// `fcntl64` tells whether this is the `fcntl64` system call.
pub fn do_fcntl(
	fd: c_int,
	cmd: c_int,
	arg: *mut c_void,
	_fcntl64: bool,
	fds: &mut FileDescriptorTable,
) -> EResult<usize> {
	match cmd {
		F_DUPFD => {
			let (id, _) = fds.duplicate_fd(fd as _, NewFDConstraint::Min(arg as _), false)?;
			Ok(id as _)
		}
		F_GETFD => {
			let fd = fds.get_fd(fd)?;
			Ok(fd.flags as _)
		}
		F_SETFD => {
			let fd = fds.get_fd_mut(fd)?;
			fd.flags = arg as _;
			Ok(0)
		}
		F_GETFL => Ok(fds.get_fd(fd)?.get_file().get_flags() as _),
		F_SETFL => {
			fds.get_fd(fd)?.get_file().set_flags(arg as _, true);
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
		F_DUPFD_CLOEXEC => {
			let (id, _) = fds.duplicate_fd(fd, NewFDConstraint::Min(arg as _), true)?;
			Ok(id as _)
		}
		F_SETPIPE_SZ => {
			// TODO
			todo!();
		}
		F_GETPIPE_SZ => {
			let file = fds.get_fd(fd)?.get_file();
			match file.get_buffer::<PipeBuffer>() {
				Some(fifo) => Ok(fifo.get_capacity() as _),
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

pub fn fcntl(
	Args((fd, cmd, arg)): Args<(c_int, c_int, *mut c_void)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_fcntl(fd, cmd, arg, false, &mut fds.lock())
}
