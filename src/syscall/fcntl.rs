//! The `fcntl` syscall call allows to manipulate a file descriptor.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::file::fd::NewFDConstraint;
use crate::process::Process;
use crate::process::regs::Regs;

// TODO Put the correct ids for each commands

/// TODO doc
const F_ADD_SEALS: i32 = 0;
/// TODO doc
const F_DUPFD: i32 = 1;
/// TODO doc
const F_DUPFD_CLOEXEC: i32 = 2;
/// TODO doc
const F_GETFD: i32 = 3;
/// TODO doc
const F_GETFL: i32 = 4;
/// TODO doc
const F_GETLEASE: i32 = 5;
/// TODO doc
const F_GETLK: i32 = 6;
/// TODO doc
const F_GETLK64: i32 = 7;
/// TODO doc
const F_GETOWN: i32 = 8;
/// TODO doc
const F_GETOWN_EX: i32 = 9;
/// TODO doc
const F_GETPIPE_SZ: i32 = 10;
/// TODO doc
const F_GETSIG: i32 = 11;
/// TODO doc
const F_GET_FILE_RW_HINT: i32 = 12;
/// TODO doc
const F_GET_RW_HINT: i32 = 13;
/// TODO doc
const F_GET_SEALS: i32 = 14;
/// TODO doc
const F_NOTIFY: i32 = 15;
/// TODO doc
const F_OFD_GETLK: i32 = 16;
/// TODO doc
const F_OFD_SETLK: i32 = 17;
/// TODO doc
const F_OFD_SETLKW: i32 = 18;
/// TODO doc
const F_OWNER_PGRP: i32 = 19;
/// TODO doc
const F_OWNER_PID: i32 = 20;
/// TODO doc
const F_OWNER_TID: i32 = 21;
/// TODO doc
const F_RDLCK: i32 = 22;
/// TODO doc
const F_SEAL_FUTURE_WRITE: i32 = 23;
/// TODO doc
const F_SEAL_GROW: i32 = 24;
/// TODO doc
const F_SEAL_SEAL: i32 = 25;
/// TODO doc
const F_SEAL_SHRINK: i32 = 26;
/// TODO doc
const F_SEAL_WRITE: i32 = 27;
/// TODO doc
const F_SETFD: i32 = 28;
/// TODO doc
const F_SETFL: i32 = 29;
/// TODO doc
const F_SETLEASE: i32 = 30;
/// TODO doc
const F_SETLK: i32 = 31;
/// TODO doc
const F_SETLK64: i32 = 32;
/// TODO doc
const F_SETLKW: i32 = 33;
/// TODO doc
const F_SETLKW64: i32 = 34;
/// TODO doc
const F_SETOWN: i32 = 35;
/// TODO doc
const F_SETOWN_EX: i32 = 36;
/// TODO doc
const F_SETPIPE_SZ: i32 = 37;
/// TODO doc
const F_SETSIG: i32 = 38;
/// TODO doc
const F_SET_FILE_RW_HINT: i32 = 39;
/// TODO doc
const F_SET_RW_HINT: i32 = 40;
/// TODO doc
const F_UNLCK: i32 = 41;
/// TODO doc
const F_WRLCK: i32 = 42;

/// Performs the fcntl system call.
/// `fcntl64` tells whether this is the fcntl64 system call.
pub fn do_fcntl(fd: i32, cmd: i32, arg: *mut c_void, _fcntl64: bool) -> Result<i32, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let mut proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	//crate::println!("fcntl: {} {} {:p} {}", fd, cmd, arg, _fcntl64); // TODO rm

	match cmd {
		F_ADD_SEALS => {
			// TODO
			Ok(0)
		},

		F_DUPFD => {
			Ok(proc.duplicate_fd(fd as _, NewFDConstraint::Min(arg as _), false)?.get_id() as _)
		},

		F_DUPFD_CLOEXEC => {
			Ok(proc.duplicate_fd(fd as _, NewFDConstraint::Min(arg as _), true)?.get_id() as _)
		},

		F_GETFD => {
			Ok(proc.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?.get_flags())
		},

		F_GETFL => {
			// TODO
			Ok(0)
		},

		F_GETLEASE => {
			// TODO
			Ok(0)
		},

		F_GETLK => {
			// TODO
			Ok(0)
		},

		F_GETLK64 => {
			// TODO
			Ok(0)
		},

		F_GETOWN => {
			// TODO
			Ok(0)
		},

		F_GETOWN_EX => {
			// TODO
			Ok(0)
		},

		F_GETPIPE_SZ => {
			// TODO
			Ok(0)
		},

		F_GETSIG => {
			// TODO
			Ok(0)
		},

		F_GET_FILE_RW_HINT => {
			// TODO
			Ok(0)
		},

		F_GET_RW_HINT => {
			// TODO
			Ok(0)
		},

		F_GET_SEALS => {
			// TODO
			Ok(0)
		},

		F_NOTIFY => {
			// TODO
			Ok(0)
		},

		F_OFD_GETLK => {
			// TODO
			Ok(0)
		},

		F_OFD_SETLK => {
			// TODO
			Ok(0)
		},

		F_OFD_SETLKW => {
			// TODO
			Ok(0)
		},

		F_OWNER_PGRP => {
			// TODO
			Ok(0)
		},

		F_OWNER_PID => {
			// TODO
			Ok(0)
		},

		F_OWNER_TID => {
			// TODO
			Ok(0)
		},

		F_RDLCK => {
			// TODO
			Ok(0)
		},

		F_SEAL_FUTURE_WRITE => {
			// TODO
			Ok(0)
		},

		F_SEAL_GROW => {
			// TODO
			Ok(0)
		},

		F_SEAL_SEAL => {
			// TODO
			Ok(0)
		},

		F_SEAL_SHRINK => {
			// TODO
			Ok(0)
		},

		F_SEAL_WRITE => {
			// TODO
			Ok(0)
		},

		F_SETFD => {
			// TODO
			Ok(0)
		},

		F_SETFL => {
			// TODO
			Ok(0)
		},

		F_SETLEASE => {
			// TODO
			Ok(0)
		},

		F_SETLK => {
			// TODO
			Ok(0)
		},

		F_SETLK64 => {
			// TODO
			Ok(0)
		},

		F_SETLKW => {
			// TODO
			Ok(0)
		},

		F_SETLKW64 => {
			// TODO
			Ok(0)
		},

		F_SETOWN => {
			// TODO
			Ok(0)
		},

		F_SETOWN_EX => {
			// TODO
			Ok(0)
		},

		F_SETPIPE_SZ => {
			// TODO
			Ok(0)
		},

		F_SETSIG => {
			// TODO
			Ok(0)
		},

		F_SET_FILE_RW_HINT => {
			// TODO
			Ok(0)
		},

		F_SET_RW_HINT => {
			// TODO
			Ok(0)
		},

		F_UNLCK => {
			// TODO
			Ok(0)
		},

		F_WRLCK => {
			// TODO
			Ok(0)
		},

		_ => Err(errno!(EINVAL)),
	}
}

/// The implementation of the `fcntl` syscall.
pub fn fcntl(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let cmd = regs.ecx as i32;
	let arg = regs.edx as *mut c_void;

	do_fcntl(fd, cmd, arg, false)
}
