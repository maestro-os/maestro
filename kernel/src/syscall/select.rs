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
	process::{
		mem_space::{
			copy::{SyscallPtr, SyscallSlice},
			MemSpace,
		},
		scheduler, Process,
	},
	syscall::{poll, Args},
	time::{
		clock,
		clock::CLOCK_MONOTONIC,
		unit::{TimeUnit, Timeval},
	},
};
use core::{
	cmp::min,
	ffi::{c_int, c_long},
};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};

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
	readfds: SyscallPtr<FDSet>,
	writefds: SyscallPtr<FDSet>,
	exceptfds: SyscallPtr<FDSet>,
	timeout: SyscallPtr<T>,
	_sigmask: Option<SyscallSlice<u8>>,
) -> EResult<usize> {
	// Get start timestamp
	let start = clock::current_time_struct::<T>(CLOCK_MONOTONIC)?;
	// Get timeout
	let timeout = timeout.copy_from_user()?.unwrap_or_default();
	// Tells whether the syscall immediately returns
	let polling = timeout.is_zero();
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
				mask |= poll::POLLIN;
			}
			if write {
				mask |= poll::POLLOUT;
			}
			if except {
				mask |= poll::POLLPRI;
			}
			if mask != 0 {
				all_zeros = false;
			}
			// Poll file
			let result = {
				// Get file descriptor
				let fds = fds.lock();
				let Ok(fd) = fds.get_fd(fd_id as _) else {
					if mask != 0 {
						return Err(errno!(EBADF));
					}
					continue;
				};
				// Get file
				let open_file_mutex = fd.get_open_file();
				let mut open_file = open_file_mutex.lock();
				// Poll
				open_file.poll(mask)?
			};
			// Set results
			let read = read && result & poll::POLLIN != 0;
			let write = write && result & poll::POLLOUT != 0;
			let except = except && result & poll::POLLPRI != 0;
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
		let curr = clock::current_time_struct::<T>(CLOCK_MONOTONIC)?;
		// On timeout, return 0
		if curr >= end {
			break 0;
		}
		// TODO Make the process sleep?
		scheduler::end_tick();
	};
	// Write back
	if let Some(val) = readfds_set {
		readfds.copy_to_user(val)?;
	}
	if let Some(val) = writefds_set {
		writefds.copy_to_user(val)?;
	}
	if let Some(val) = exceptfds_set {
		exceptfds.copy_to_user(val)?;
	}
	Ok(res)
}

#[allow(clippy::type_complexity)]
pub fn select(
	Args((nfds, readfds, writefds, exceptfds, timeout)): Args<(
		c_int,
		SyscallPtr<FDSet>,
		SyscallPtr<FDSet>,
		SyscallPtr<FDSet>,
		SyscallPtr<Timeval>,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_select(fds, nfds as _, readfds, writefds, exceptfds, timeout, None)
}
