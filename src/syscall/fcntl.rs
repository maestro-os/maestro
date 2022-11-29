//! The `fcntl` syscall call allows to manipulate a file descriptor.

use crate::errno::Errno;
use crate::file::fd::NewFDConstraint;
use crate::file::FileContent;
use crate::process::Process;
use core::ffi::c_int;
use core::ffi::c_void;
use macros::syscall;

/// TODO doc
const F_DUPFD: i32 = 0;
/// TODO doc
const F_GETFD: i32 = 1;
/// TODO doc
const F_SETFD: i32 = 2;
/// TODO doc
const F_GETFL: i32 = 3;
/// TODO doc
const F_SETFL: i32 = 4;
/// TODO doc
const F_GETLK: i32 = 5;
/// TODO doc
const F_SETLK: i32 = 6;
/// TODO doc
const F_SETLKW: i32 = 7;
/// TODO doc
const F_SETOWN: i32 = 8;
/// TODO doc
const F_GETOWN: i32 = 9;
/// TODO doc
const F_SETSIG: i32 = 10;
/// TODO doc
const F_GETSIG: i32 = 11;
/// TODO doc
const F_GETLK64: i32 = 12;
/// TODO doc
const F_SETLK64: i32 = 13;
/// TODO doc
const F_SETLKW64: i32 = 14;
/// TODO doc
const F_SETOWN_EX: i32 = 15;
/// TODO doc
const F_GETOWN_EX: i32 = 16;
/// TODO doc
const F_OFD_GETLK: i32 = 36;
/// TODO doc
const F_OFD_SETLK: i32 = 37;
/// TODO doc
const F_OFD_SETLKW: i32 = 38;
/// TODO doc
const F_SETLEASE: i32 = 1024;
/// TODO doc
const F_GETLEASE: i32 = 1025;
/// TODO doc
const F_NOTIFY: i32 = 1026;
/// TODO doc
const F_DUPFD_CLOEXEC: i32 = 1030;
/// TODO doc
const F_SETPIPE_SZ: i32 = 1031;
/// TODO doc
const F_GETPIPE_SZ: i32 = 1032;
/// TODO doc
const F_ADD_SEALS: i32 = 1033;
/// TODO doc
const F_GET_SEALS: i32 = 1034;
/// TODO doc
const F_GET_RW_HINT: i32 = 1035;
/// TODO doc
const F_SET_RW_HINT: i32 = 1036;
/// TODO doc
const F_GET_FILE_RW_HINT: i32 = 1037;
/// TODO doc
const F_SET_FILE_RW_HINT: i32 = 1038;

/// TODO doc
const F_SEAL_FUTURE_WRITE: i32 = 16;

/// TODO doc
const F_RDLCK: i32 = 0;
/// TODO doc
const F_WRLCK: i32 = 1;
/// TODO doc
const F_UNLCK: i32 = 2;

/// TODO doc
const F_OWNER_PGRP: i32 = 2;
/// TODO doc
const F_OWNER_PID: i32 = 1;
/// TODO doc
const F_OWNER_TID: i32 = 0;

/// TODO doc
const F_SEAL_GROW: i32 = 4;
/// TODO doc
const F_SEAL_SEAL: i32 = 1;
/// TODO doc
const F_SEAL_SHRINK: i32 = 2;
/// TODO doc
const F_SEAL_WRITE: i32 = 8;

/// Performs the fcntl system call.
/// `fcntl64` tells whether this is the fcntl64 system call.
pub fn do_fcntl(fd: i32, cmd: i32, arg: *mut c_void, _fcntl64: bool) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let fds_mutex = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get_mut();

		proc.get_fds().unwrap()
	};
	let fds_guard = fds_mutex.lock();
	let fds = fds_guard.get_mut();

	//crate::println!("fcntl: {} {} {:p} {}", fd, cmd, arg, _fcntl64); // TODO rm

	match cmd {
		F_DUPFD => Ok(fds
			.duplicate_fd(fd as _, NewFDConstraint::Min(arg as _), false)?
			.get_id() as _),

		F_GETFD => {
			let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
			Ok(fd.get_flags())
		}

		F_SETFD => {
			fds.set_fd_flags(fd as _, arg as _)?;
			Ok(0)
		}

		F_GETFL => {
			let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
			let open_file_mutex = fd.get_open_file();
			let open_file_guard = open_file_mutex.lock();
			let open_file = open_file_guard.get();

			Ok(open_file.get_flags())
		}

		F_SETFL => {
			let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
			let open_file_mutex = fd.get_open_file();
			let open_file_guard = open_file_mutex.lock();
			let open_file = open_file_guard.get_mut();

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
			let open_file_guard = open_file_mutex.lock();
			let open_file = open_file_guard.get();

			let file_mutex = open_file.get_file()?;
			let file_guard = file_mutex.lock();
			let file = file_guard.get();

			match file.get_content() {
				FileContent::Fifo => {
					// TODO
					todo!();
				}

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

/// The implementation of the `fcntl` syscall.
#[syscall]
pub fn fcntl(fd: c_int, cmd: c_int, arg: *mut c_void) -> Result<i32, Errno> {
	do_fcntl(fd, cmd, arg, false)
}
