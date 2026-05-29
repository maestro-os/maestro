/*
 * Copyright 2026 Luc Lenôtre
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

//! `epoll` is an I/O event notification facility.

use crate::{
	file::{
		File, FileType, O_CLOEXEC, O_RDWR,
		fd::{FD_CLOEXEC, fd_to_file},
		fs::float,
		poll::{EpollEvent, EpollFileOps, EpollItem},
	},
	memory::user::{UserPtr, UserSlice},
	process,
	process::{Process, State, scheduler::schedule},
	time::{
		clock::{Clock, current_time_ms},
		unit::Timestamp,
	},
};
use core::{ffi::c_int, hint::unlikely, ptr};
use utils::{errno, errno::EResult, ptr::arc::Arc};

/// epoll event flag: associated file is available for `read` operations.
pub const EPOLLIN: u32 = 0x001;
/// epoll event flag: there is an exceptional condition on the file descriptor.
pub const EPOLLPRI: u32 = 0x002;
/// epoll event flag: associated file is available for `write` operations.
pub const EPOLLOUT: u32 = 0x004;
/// epoll event flag: error condition. Always reported, need not be requested.
pub const EPOLLERR: u32 = 0x008;
/// epoll event flag: hang up. Always reported, need not be requested.
pub const EPOLLHUP: u32 = 0x010;
/// epoll event flag: equivalent to [`EPOLLIN`].
pub const EPOLLRDNORM: u32 = 0x040;
/// epoll event flag: priority band data can be read.
pub const EPOLLRDBAND: u32 = 0x080;
/// epoll event flag: equivalent to [`EPOLLOUT`].
pub const EPOLLWRNORM: u32 = 0x100;
/// epoll event flag: priority data may be written.
pub const EPOLLWRBAND: u32 = 0x200;
/// epoll event flag: peer closed connection, or shut down the writing half.
pub const EPOLLRDHUP: u32 = 0x2000;
/// epoll behaviour flag: requests one-shot notification.
pub const EPOLLONESHOT: u32 = 1 << 30;
/// epoll behaviour flag: requests edge-triggered notification.
pub const EPOLLET: u32 = 1 << 31;

/// `epoll_ctl` operation: add an entry to the interest list.
pub const EPOLL_CTL_ADD: c_int = 1;
/// `epoll_ctl` operation: remove an entry from the interest list.
pub const EPOLL_CTL_DEL: c_int = 2;
/// `epoll_ctl` operation: change the settings of an entry in the interest list.
pub const EPOLL_CTL_MOD: c_int = 3;

/// Mask of the behaviour flags, which are not poll events.
pub const EPOLL_FLAGS: u32 = EPOLLET | EPOLLONESHOT;

/// Creates an epoll instance and returns a file descriptor referring to it.
///
/// `cloexec` tells whether the close-on-exec flag must be set on the new
/// descriptor.
fn create(cloexec: bool) -> EResult<usize> {
	let entry = float::get_entry(EpollFileOps::default(), FileType::None)?;
	let file = File::open_floating(entry, O_RDWR)?;
	let fd_flags = if cloexec { FD_CLOEXEC } else { 0 };
	let (fd, _) = Process::current()
		.file_descriptors()
		.lock()
		.create_fd(fd_flags, file)?;
	Ok(fd as _)
}

/// Returns the [`EpollFileOps`] backing the epoll file descriptor `epfd`.
///
/// If `epfd` does not refer to an epoll instance, the function returns
/// [`errno::EINVAL`].
fn get_epoll(epfd: c_int) -> EResult<Arc<File>> {
	let file = fd_to_file(epfd)?;
	// Make sure the descriptor really is an epoll instance
	if unlikely(file.get_buffer::<EpollFileOps>().is_none()) {
		return Err(errno!(EINVAL));
	}
	Ok(file)
}

/// Creates a new epoll instance.
///
/// `size` is a hint that is ignored, kept for compatibility.
pub(super) fn epoll_create(size: c_int) -> EResult<usize> {
	// Historically `size` had to be greater than zero
	if unlikely(size <= 0) {
		return Err(errno!(EINVAL));
	}
	create(false)
}

/// epoll creation flag: set the close-on-exec flag on the new descriptor.
const EPOLL_CLOEXEC: c_int = O_CLOEXEC;

pub(super) fn epoll_create1(flags: c_int) -> EResult<usize> {
	if unlikely(flags & !EPOLL_CLOEXEC != 0) {
		return Err(errno!(EINVAL));
	}
	create(flags & EPOLL_CLOEXEC != 0)
}

pub(super) fn epoll_ctl(
	epfd: c_int,
	op: c_int,
	fd: c_int,
	event: UserPtr<EpollEvent>,
) -> EResult<usize> {
	let epfile = get_epoll(epfd)?;
	let target = fd_to_file(fd)?;
	// An epoll instance cannot monitor itself
	if unlikely(ptr::eq(Arc::as_ptr(&epfile), Arc::as_ptr(&target))) {
		return Err(errno!(EINVAL));
	}
	let file_ptr = Arc::as_ptr(&target);
	let epoll = epfile.get_buffer::<EpollFileOps>().unwrap();
	let mut items = epoll.0.lock();
	match op {
		EPOLL_CTL_ADD => {
			if unlikely(items.contains_key(&file_ptr)) {
				return Err(errno!(EEXIST));
			}
			let event = event.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
			items.insert(
				file_ptr,
				EpollItem {
					file: target,
					events: event.events & !EPOLL_FLAGS,
					flags: event.events & EPOLL_FLAGS,
					data: event.data,
					reported: 0,
				},
			)?;
		}
		EPOLL_CTL_MOD => {
			let event = event.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
			let item = items.get_mut(&file_ptr).ok_or_else(|| errno!(ENOENT))?;
			item.events = event.events & !EPOLL_FLAGS;
			item.flags = event.events & EPOLL_FLAGS;
			item.data = event.data;
			// Re-arm: allow events to be reported again
			item.reported = 0;
		}
		EPOLL_CTL_DEL => {
			items.remove(&file_ptr);
		}
		_ => return Err(errno!(EINVAL)),
	}
	Ok(0)
}

fn collect_ready(epoll: &EpollFileOps, events: UserSlice<EpollEvent>) -> EResult<usize> {
	let mut items = epoll.0.lock();
	let mut count = 0;
	let iter = items.iter_mut().take(events.len());
	for (i, (_, item)) in iter.enumerate() {
		// A disabled (consumed one-shot) entry is skipped
		if item.events == 0 {
			continue;
		}
		// `EPOLLERR` and `EPOLLHUP` are always reported
		let mask = item.events | EPOLLERR | EPOLLHUP;
		let ready = item.file.ops.poll(&item.file, mask)? & mask;
		if item.flags & EPOLLET != 0 {
			// Edge-triggered: only report newly-ready events
			let new = ready & !item.reported;
			item.reported = ready;
			if new == 0 {
				continue;
			}
		} else if ready == 0 {
			continue;
		}
		events.copy_to_user(
			i,
			&[EpollEvent {
				events: ready,
				data: item.data,
			}],
		)?;
		// One-shot: disable the entry until it is re-armed with `EPOLL_CTL_MOD`
		if item.flags & EPOLLONESHOT != 0 {
			item.events = 0;
		}
		count += 1;
	}
	Ok(count)
}

pub(super) fn epoll_wait(
	epfd: c_int,
	events: *mut EpollEvent,
	maxevents: c_int,
	timeout: c_int,
) -> EResult<usize> {
	if maxevents <= 0 {
		return Err(errno!(EINVAL));
	}
	let events = UserSlice::from_user(events, maxevents as usize)?;
	let epfile = get_epoll(epfd)?;
	let epoll = epfile.get_buffer::<EpollFileOps>().unwrap();
	// `None` means no timeout
	let to = (timeout >= 0).then_some(timeout as Timestamp);
	let start_ts = current_time_ms(Clock::Monotonic);
	loop {
		let count = collect_ready(epoll, events)?;
		if count > 0 {
			return Ok(count);
		}
		// Timeout
		match to {
			Some(0) => return Ok(0),
			Some(timeout) => {
				if current_time_ms(Clock::Monotonic) >= start_ts + timeout {
					return Ok(0);
				}
			}
			_ => {}
		}
		// Interrupted by a signal
		if Process::current().has_pending_signal() {
			return Err(errno!(EINTR));
		}
		// Wait for completion
		process::set_state(State::Sleeping);
		schedule();
	}
}
