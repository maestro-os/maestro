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

//! The `poll` system call allows to wait for events on a given set of file
//! descriptors.

use crate::{
	process::{mem_space::copy::SyscallSlice, scheduler, Process},
	syscall::Args,
	time::{
		clock,
		clock::CLOCK_MONOTONIC,
		unit::{Timestamp, TimestampScale},
	},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

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
	Args((fds, nfds, timeout)): Args<(SyscallSlice<PollFD>, usize, c_int)>,
) -> EResult<usize> {
	// The timeout. `None` means no timeout
	let to = (timeout >= 0).then_some(timeout as Timestamp);
	// The start timestamp
	let start_ts = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Millisecond)?;
	loop {
		// Check whether the system call timed out
		if let Some(timeout) = to {
			let now = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Millisecond)?;
			if now >= start_ts + timeout {
				return Ok(0);
			}
		}
		{
			let fds_arr = fds.copy_from_user(..nfds)?.ok_or_else(|| errno!(EFAULT))?;
			// Check the file descriptors list
			for fd in &fds_arr {
				if fd.events as u32 & POLLIN != 0 {
					// TODO
					todo!();
				}
				if fd.events as u32 & POLLPRI != 0 {
					// TODO
					todo!();
				}
				if fd.events as u32 & POLLOUT != 0 {
					// TODO
					todo!();
				}
				if fd.events as u32 & POLLRDNORM != 0 {
					// TODO
					todo!();
				}
				if fd.events as u32 & POLLRDBAND != 0 {
					// TODO
					todo!();
				}
				if fd.events as u32 & POLLWRNORM != 0 {
					// TODO
					todo!();
				}
				if fd.events as u32 & POLLWRBAND != 0 {
					// TODO
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
		scheduler::end_tick();
	}
}
