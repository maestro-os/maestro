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

//! `poll` and `select` make the calling process wait until something happen on a file descriptor.

use crate::{
	arch::x86::{hlt, sti},
	file::poll::{
		FD_SETSIZE, FDSet, POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT, POLLPRI, PollFD,
	},
	memory::user::{UserPtr, UserSlice},
	process::Process,
	time::{
		clock::{Clock, current_time_ns},
		unit::{TimeUnit, Timespec, Timestamp, Timeval},
	},
};
use core::{cmp::min, ffi::c_int};
use utils::{errno, errno::EResult};

/// Polls the file descriptor `fd` of the current process for the events in `mask`.
///
/// Returns the set of events that occurred, or `None` if `fd` does not refer to an open file.
fn poll_fd(fd: c_int, mask: u32) -> EResult<Option<u32>> {
	let file = {
		let fds = Process::current().file_descriptors();
		let fds = fds.lock();
		let Ok(fd) = fds.get_fd(fd) else {
			return Ok(None);
		};
		fd.get_file().clone()
	};
	Ok(Some(file.ops.poll(&file, mask)?))
}

/// Waits for events on a set of file descriptors, shared by `poll` and `select`.
fn wait_events<F: FnMut() -> EResult<usize>>(
	deadline: Option<Timestamp>,
	mut scan: F,
) -> EResult<usize> {
	loop {
		let count = scan()?;
		if count > 0 {
			return Ok(count);
		}
		if let Some(deadline) = deadline {
			if current_time_ns(Clock::Monotonic) >= deadline {
				return Ok(0);
			}
		}
		if Process::current().has_pending_signal() {
			return Err(errno!(EINTR));
		}
		// TODO make process sleep
		sti();
		hlt();
	}
}

/// Performs the select operation.
///
/// Arguments:
/// - `nfds` is the number of the highest checked fd + 1.
/// - `readfds` is the bitfield of fds to check for read operations.
/// - `writefds` is the bitfield of fds to check for write operations.
/// - `exceptfds` is the bitfield of fds to check for exceptional conditions.
/// - `timeout` is the timeout after which the syscall returns.
/// - `sigmask` TODO
pub fn do_select<T: TimeUnit>(
	nfds: u32,
	readfds: UserPtr<FDSet>,
	writefds: UserPtr<FDSet>,
	exceptfds: UserPtr<FDSet>,
	timeout: UserPtr<T>,
	_sigmask: Option<*mut u8>,
) -> EResult<usize> {
	// Deadline in nanoseconds. `None` means block indefinitely
	let deadline = timeout
		.copy_from_user()?
		.map(|t| {
			let now = current_time_ns(Clock::Monotonic);
			now.checked_add(t.to_nano()).ok_or_else(|| errno!(EINVAL))
		})
		.transpose()?;
	let read_in = readfds.copy_from_user()?;
	let write_in = writefds.copy_from_user()?;
	let except_in = exceptfds.copy_from_user()?;
	crate::dbgs!(
		"[DBG select] nfds={nfds} pollfd0={:?} inputs={} echoes={} lflag={:#x}",
		poll_fd(0, POLLIN),
		crate::TTY_INPUT_CALLS.load(core::sync::atomic::Ordering::Relaxed),
		crate::TTY_ECHO_CALLS.load(core::sync::atomic::Ordering::Relaxed),
		crate::TTY_LAST_LFLAG.load(core::sync::atomic::Ordering::Relaxed)
	);
	let mut read_out = read_in.as_ref().map(|_| FDSet::default());
	let mut write_out = write_in.as_ref().map(|_| FDSet::default());
	let mut except_out = except_in.as_ref().map(|_| FDSet::default());
	let nfds = min(nfds, FD_SETSIZE as u32);
	let res = wait_events(deadline, || {
		let mut count = 0;
		for fd_id in 0..nfds {
			let read = read_in.as_ref().is_some_and(|s| s.is_set(fd_id));
			let write = write_in.as_ref().is_some_and(|s| s.is_set(fd_id));
			let except = except_in.as_ref().is_some_and(|s| s.is_set(fd_id));
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
			if mask == 0 {
				continue;
			}
			let result = poll_fd(fd_id as _, mask)?.ok_or_else(|| errno!(EBADF))?;
			let read = read && result & POLLIN != 0;
			let write = write && result & POLLOUT != 0;
			let except = except && result & POLLPRI != 0;
			if read {
				read_out.as_mut().unwrap().set(fd_id, true);
			}
			if write {
				write_out.as_mut().unwrap().set(fd_id, true);
			}
			if except {
				except_out.as_mut().unwrap().set(fd_id, true);
			}
			count += read as usize + write as usize + except as usize;
		}
		Ok(count)
	})?;
	if let Some(val) = read_out {
		readfds.copy_to_user(&val)?;
	}
	if let Some(val) = write_out {
		writefds.copy_to_user(&val)?;
	}
	if let Some(val) = except_out {
		exceptfds.copy_to_user(&val)?;
	}
	Ok(res)
}

#[allow(clippy::type_complexity)]
pub(super) fn select(
	nfds: c_int,
	readfds: UserPtr<FDSet>,
	writefds: UserPtr<FDSet>,
	exceptfds: UserPtr<FDSet>,
	timeout: UserPtr<Timeval>,
) -> EResult<usize> {
	do_select(nfds as _, readfds, writefds, exceptfds, timeout, None)
}

#[allow(clippy::type_complexity)]
pub(super) fn _newselect(
	nfds: c_int,
	readfds: UserPtr<FDSet>,
	writefds: UserPtr<FDSet>,
	exceptfds: UserPtr<FDSet>,
	timeout: UserPtr<Timeval>,
) -> EResult<usize> {
	do_select(nfds as _, readfds, writefds, exceptfds, timeout, None)
}

#[allow(clippy::type_complexity)]
pub(super) fn pselect6(
	nfds: c_int,
	readfds: UserPtr<FDSet>,
	writefds: UserPtr<FDSet>,
	exceptfds: UserPtr<FDSet>,
	timeout: UserPtr<Timespec>,
	sigmask: *mut u8,
) -> EResult<usize> {
	do_select(
		nfds as _,
		readfds,
		writefds,
		exceptfds,
		timeout,
		Some(sigmask),
	)
}

pub(super) fn poll(fds: *mut PollFD, nfds: usize, timeout: c_int) -> EResult<usize> {
	let fds = UserSlice::from_user(fds, nfds)?;
	let deadline = (timeout >= 0).then(|| {
		let now = current_time_ns(Clock::Monotonic);
		now.saturating_add(timeout as Timestamp * 1_000_000)
	});
	let mut arr = fds.copy_from_user_vec(0)?.ok_or_else(|| errno!(EFAULT))?;
	crate::dbgs!(
		"[DBG poll] nfds={nfds} timeout={timeout} fd0={:?}",
		arr.first().map(|f| (f.fd, f.events))
	);
	let res = wait_events(deadline, || {
		let mut count = 0;
		for fd in &mut arr {
			let requested = fd.events as u32;
			// `POLLERR` and `POLLHUP` are always reported
			let mask = requested | POLLERR | POLLHUP;
			let revents = if fd.fd < 0 {
				// Negative file descriptors are ignored
				0
			} else {
				match poll_fd(fd.fd, mask)? {
					Some(result) => result & mask,
					// `poll` reports an invalid fd through `POLLNVAL` rather than failing
					None => POLLNVAL,
				}
			};
			fd.revents = revents as _;
			if revents != 0 {
				count += 1;
			}
		}
		Ok(count)
	})?;
	fds.copy_to_user(0, &arr)?;
	Ok(res)
}
