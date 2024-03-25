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

//! The ioctl syscall allows to control a device represented by a file
//! descriptor.

use crate::process::Process;
use core::ffi::{c_int, c_ulong, c_void};
use macros::syscall;
use utils::{errno, errno::Errno};

// ioctl requests: hard drive

/// ioctl request: get device geometry.
pub const HDIO_GETGEO: u32 = 0x00000301;

// ioctl requests: storage

/// ioctl request: re-read partition table.
pub const BLKRRPART: u32 = 0x0000125f;
/// ioctl request: get block size.
pub const BLKSSZGET: u32 = 0x00001268;
/// ioctl request: get storage size in bytes.
pub const BLKGETSIZE64: u32 = 0x00001272;

// ioctl requests: TTY

/// ioctl request: Returns the current serial port settings.
pub const TCGETS: u32 = 0x00005401;
/// ioctl request: Sets the serial port settings. Making the change immediately.
pub const TCSETS: u32 = 0x00005402;
/// ioctl request: Sets the serial port settings. Making the change only when
/// all currently written data has been transmitted. At this points, any
/// received data is discarded.
pub const TCSETSW: u32 = 0x00005403;
/// ioctl request: Sets the serial port settings. Making the change only when
/// all currently written data has been transmitted.
pub const TCSETSF: u32 = 0x00005404;
/// ioctl request: Get the foreground process group ID on the terminal.
pub const TIOCGPGRP: u32 = 0x0000540f;
/// ioctl request: Set the foreground process group ID on the terminal.
pub const TIOCSPGRP: u32 = 0x00005410;
/// ioctl request: Returns the window size of the terminal.
pub const TIOCGWINSZ: u32 = 0x00005413;
/// ioctl request: Sets the window size of the terminal.
pub const TIOCSWINSZ: u32 = 0x00005414;
/// ioctl request: Returns the number of bytes available on the file descriptor.
pub const FIONREAD: u32 = 0x0000541b;

/// Enumeration of IO directions for ioctl requests.
#[derive(Eq, PartialEq)]
pub enum Direction {
	/// No data to be transferred.
	None,
	/// The userspace requires information.
	Read,
	/// The userspace transmits information.
	Write,
}

impl TryFrom<c_ulong> for Direction {
	type Error = ();

	fn try_from(n: c_ulong) -> Result<Self, Self::Error> {
		match n {
			0 => Ok(Self::None),
			2 => Ok(Self::Read),
			1 => Ok(Self::Write),

			_ => Err(()),
		}
	}
}

/// Structure representing an `ioctl` request.
pub struct Request {
	/// Major number of the request.
	pub major: u8,
	/// Minor number of the request.
	pub minor: u8,

	/// The size of the data treated by the request in bytes.
	pub size: usize,
	/// Tells whether IO direction of the ioctl request.
	pub direction: Direction,
}

impl From<c_ulong> for Request {
	fn from(req: c_ulong) -> Self {
		Self {
			major: ((req >> 8) & 0xff) as u8,
			minor: (req & 0xff) as u8,

			size: ((req >> 16) & 0x3f) as usize,
			direction: ((req >> 30) & 0x03).try_into().unwrap(),
		}
	}
}

impl Request {
	/// Returns the value as the old format for ioctl.
	pub fn get_old_format(&self) -> c_ulong {
		((self.major as u32) << 8) | self.minor as u32
	}
}

#[syscall]
pub fn ioctl(fd: c_int, request: c_ulong, argp: *const c_void) -> Result<i32, Errno> {
	let request = Request::from(request);

	// Getting the memory space and file
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

	// Getting the device file
	let mut open_file = open_file_mutex.lock();

	// Executing ioctl with the current memory space
	let ret = open_file.ioctl(mem_space, request, argp)?;

	Ok(ret as _)
}
