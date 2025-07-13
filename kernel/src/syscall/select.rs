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

//! `select` waits for a file descriptor in the given sets to be readable,
//! writable or for an exception to occur.

use crate::{
	file::fd::FileDescriptorTable,
	memory::user::{UserPtr, UserSlice},
	process::scheduler::schedule,
	sync::mutex::Mutex,
	syscall::Args,
	time::{
		clock::{Clock, current_time_ms, current_time_ns},
		unit::{TimeUnit, Timespec, Timestamp, Timeval},
	},
};
use core::{
	cmp::min,
	ffi::{c_int, c_long},
};
use utils::{errno, errno::EResult, ptr::arc::Arc};

/// The number of file descriptors in FDSet.
pub const FD_SETSIZE: usize = 1024;

/// Structure representing `fd_set`.
#[repr(C)]
#[derive(Debug)]
pub struct FDSet {
	/// The set's bitfield.
	fds_bits: [c_long; FD_SETSIZE / c_long::BITS as usize],
}

impl FDSet {
	/// Tells whether the given file descriptor `fd` is set in the list.
	fn is_set(&self, fd: u32) -> bool {
		if fd as usize >= FD_SETSIZE {
			return false;
		}
		// TODO Check correctness
		let i = (fd as usize) / c_long::BITS as usize;
		self.fds_bits[i] >> (fd % c_long::BITS) != 0
	}

	/// Sets or clears the bit for file descriptor `fd`.
	fn set(&mut self, fd: u32, val: bool) {
		// TODO Check correctness
		let i = (fd as usize) / c_long::BITS as usize;
		if val {
			self.fds_bits[i] |= 1 << (fd % c_long::BITS);
		} else {
			self.fds_bits[i] &= !(1 << (fd % c_long::BITS));
		}
	}
}

/// Performs the select operation.
///
/// Arguments:
/// - `mem_space` is the process's memory space.
/// - `fds` is the process's file descriptors table.
/// - `nfds` is the number of the highest checked fd + 1.
/// - `readfds` is the bitfield of fds to check for read operations.
/// - `writefds` is the bitfield of fds to check for write operations.
/// - `exceptfds` is the bitfield of fds to check for exceptional conditions.
/// - `timeout` is the timeout after which the syscall returns.
/// - `sigmask` TODO
pub fn do_select<T: TimeUnit>(
	fds: Arc<Mutex<FileDescriptorTable>>,
	nfds: u32,
	readfds: UserPtr<FDSet>,
	writefds: UserPtr<FDSet>,
	exceptfds: UserPtr<FDSet>,
	timeout: UserPtr<T>,
	_sigmask: Option<*mut u8>,
) -> EResult<usize> {
	let start = current_time_ns(Clock::Monotonic);
	// Get timeout
	let timeout = timeout
		.copy_from_user()?
		.map(|t| t.to_nano())
		.unwrap_or_default();
	// Tells whether the syscall immediately returns
	let polling = timeout == 0;
	// The end timestamp
	let end = start + timeout;
	// Read
	let mut readfds_set = readfds.copy_from_user()?;
	let mut writefds_set = writefds.copy_from_user()?;
	let mut exceptfds_set = exceptfds.copy_from_user()?;
	let res = loop {
		let mut events_count = 0;
		// Set if every bitfields are set to zero
		let mut all_zeros = true;
		for fd_id in 0..min(nfds, FD_SETSIZE as u32) {
			let read = readfds_set
				.as_ref()
				.map(|fds| fds.is_set(fd_id))
				.unwrap_or(false);
			let write = writefds_set
				.as_ref()
				.map(|fds| fds.is_set(fd_id))
				.unwrap_or(false);
			let except = exceptfds_set
				.as_ref()
				.map(|fds| fds.is_set(fd_id))
				.unwrap_or(false);
			// Build event mask
			let mut mask = 0;
			if read {
				mask |= POLLIN;
			}
			if write {
				mask |= POLLOUT;
			}
			if except {
				mask |= POLLPRI;
			}
			if mask != 0 {
				all_zeros = false;
			}
			// Poll file
			let result = {
				let fds = fds.lock();
				let Ok(fd) = fds.get_fd(fd_id as _) else {
					if mask != 0 {
						return Err(errno!(EBADF));
					}
					continue;
				};
				let file = fd.get_file();
				file.ops.poll(file, mask)?
			};
			// Set results
			let read = read && result & POLLIN != 0;
			let write = write && result & POLLOUT != 0;
			let except = except && result & POLLPRI != 0;
			if let Some(fds) = &mut readfds_set {
				fds.set(fd_id, read);
			}
			if let Some(fds) = &mut writefds_set {
				fds.set(fd_id, write);
			}
			if let Some(fds) = &mut exceptfds_set {
				fds.set(fd_id, except);
			}
			events_count += read as usize + write as usize + except as usize;
		}
		// If one or more events occurred, return
		if all_zeros || polling || events_count > 0 {
			break events_count;
		}
		let ts = current_time_ns(Clock::Monotonic);
		// On timeout, return 0
		if ts >= end {
			break 0;
		}
		// TODO Make the process sleep?
		schedule();
	};
	// Write back
	if let Some(val) = readfds_set {
		readfds.copy_to_user(&val)?;
	}
	if let Some(val) = writefds_set {
		writefds.copy_to_user(&val)?;
	}
	if let Some(val) = exceptfds_set {
		exceptfds.copy_to_user(&val)?;
	}
	Ok(res)
}

#[allow(clippy::type_complexity)]
pub(super) fn select(
	Args((nfds, readfds, writefds, exceptfds, timeout)): Args<(
		c_int,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<Timeval>,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_select(fds, nfds as _, readfds, writefds, exceptfds, timeout, None)
}

#[allow(clippy::type_complexity)]
pub(super) fn _newselect(
	Args((nfds, readfds, writefds, exceptfds, timeout)): Args<(
		c_int,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<Timeval>,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_select(fds, nfds as _, readfds, writefds, exceptfds, timeout, None)
}

#[allow(clippy::type_complexity)]
pub(super) fn pselect6(
	Args((nfds, readfds, writefds, exceptfds, timeout, sigmask)): Args<(
		c_int,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<Timespec>,
		*mut u8,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_select(
		fds,
		nfds as _,
		readfds,
		writefds,
		exceptfds,
		timeout,
		Some(sigmask),
	)
}

/// Poll event: There is data to read.
pub const POLLIN: u32 = 0x1;
/// Poll event: There is some exceptional condition on the file descriptor.
pub const POLLPRI: u32 = 0x2;
/// Poll event: Writing is now possible.
pub const POLLOUT: u32 = 0x4;
/// Poll event: Error condition.
pub const POLLERR: u32 = 0x8;
/// Poll event: Hang up.
pub const POLLHUP: u32 = 0x10;
/// Poll event: Invalid request.
pub const POLLNVAL: u32 = 0x20;
/// Poll event: Equivalent to POLLIN.
pub const POLLRDNORM: u32 = 0x40;
/// Poll event: Priority band data can be read.
pub const POLLRDBAND: u32 = 0x80;
/// Poll event: Equivalent to POLLOUT.
pub const POLLWRNORM: u32 = 0x100;
/// Poll event: Priority data may be written.
pub const POLLWRBAND: u32 = 0x200;
/// Poll event: Stream socket peer closed connection, or shut down writing half
/// of connection.
pub const POLLRDHUP: u32 = 0x2000;

/// A file descriptor passed to the `poll` system call.
#[repr(C)]
#[derive(Debug)]
pub struct PollFD {
	/// The file descriptor.
	fd: i32,
	/// The input mask telling which events to look for.
	events: i16,
	/// The output mask telling which events happened.
	revents: i16,
}

pub(super) fn poll(
	Args((fds, nfds, timeout)): Args<(*mut PollFD, usize, c_int)>,
) -> EResult<usize> {
	let fds = UserSlice::from_user(fds, nfds)?;
	// The timeout. `None` means no timeout
	let to = (timeout >= 0).then_some(timeout as Timestamp);
	let start_ts = current_time_ms(Clock::Monotonic);
	loop {
		// Check whether the system call timed out
		if let Some(timeout) = to {
			let now = current_time_ms(Clock::Monotonic);
			if now >= start_ts + timeout {
				return Ok(0);
			}
		}
		{
			let fds_arr = fds.copy_from_user_vec(0)?.ok_or_else(|| errno!(EFAULT))?;
			// Check the file descriptors list
			for fd in &fds_arr {
				if fd.events as u32 & POLLIN != 0 {
					todo!();
				}
				if fd.events as u32 & POLLPRI != 0 {
					todo!();
				}
				if fd.events as u32 & POLLOUT != 0 {
					todo!();
				}
				if fd.events as u32 & POLLRDNORM != 0 {
					todo!();
				}
				if fd.events as u32 & POLLRDBAND != 0 {
					todo!();
				}
				if fd.events as u32 & POLLWRNORM != 0 {
					todo!();
				}
				if fd.events as u32 & POLLWRBAND != 0 {
					todo!();
				}
			}
			// The number of file descriptor with at least one event
			let fd_event_count = fds_arr.iter().filter(|fd| fd.revents != 0).count();
			// If at least on event happened, return the number of file descriptors
			// concerned
			if fd_event_count > 0 {
				fds.copy_to_user(0, &fds_arr)?;
				return Ok(fd_event_count as _);
			}
		}
		// TODO Make process sleep until an event occurs on a file descriptor in
		// `fds`
		schedule();
	}
}
