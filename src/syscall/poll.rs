//! The `poll` system call allows to wait for events on a given set of file
//! descriptors.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::scheduler;
use crate::process::Process;
use crate::time;
use crate::time::unit::Timestamp;
use crate::time::unit::TimestampScale;
use crate::util::io;
use core::ffi::c_int;
use macros::syscall;

/// Structure representing a file descriptor passed to the `poll` system call.
#[repr(C)]
#[derive(Debug)]
struct PollFD {
	/// The file descriptor.
	fd: i32,
	/// The input mask telling which events to look for.
	events: i16,
	/// The output mask telling which events happened.
	revents: i16,
}

// TODO Check second arg type
#[syscall]
pub fn poll(fds: SyscallSlice<PollFD>, nfds: usize, timeout: c_int) -> Result<i32, Errno> {
	// The timeout. None means no timeout
	let to: Option<Timestamp> = if timeout >= 0 {
		Some(timeout as _)
	} else {
		None
	};

	// The start timestamp
	let start_ts = time::get(TimestampScale::Millisecond, true).unwrap_or(0);

	loop {
		// Checking whether the system call timed out
		if let Some(timeout) = to {
			if time::get(TimestampScale::Millisecond, true).unwrap_or(0) >= start_ts + timeout {
				return Ok(0);
			}
		}

		{
			let proc_mutex = Process::current_assert();
			let proc = proc_mutex.lock();

			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();

			let fds = fds
				.get(&mem_space_guard, nfds)?
				.ok_or_else(|| errno!(EFAULT))?;

			// Checking the file descriptors list
			for fd in fds {
				if fd.events as u32 & io::POLLIN != 0 {
					// TODO
					todo!();
				}

				if fd.events as u32 & io::POLLPRI != 0 {
					// TODO
					todo!();
				}

				if fd.events as u32 & io::POLLOUT != 0 {
					// TODO
					todo!();
				}

				if fd.events as u32 & io::POLLRDNORM != 0 {
					// TODO
					todo!();
				}

				if fd.events as u32 & io::POLLRDBAND != 0 {
					// TODO
					todo!();
				}

				if fd.events as u32 & io::POLLWRNORM != 0 {
					// TODO
					todo!();
				}

				if fd.events as u32 & io::POLLWRBAND != 0 {
					// TODO
					todo!();
				}
			}

			// The number of file descriptor with at least one event
			let fd_event_count = fds.iter().filter(|fd| fd.revents != 0).count();
			// If at least on event happened, return the number of file descriptors
			// concerned
			if fd_event_count > 0 {
				return Ok(fd_event_count as _);
			}
		}

		// TODO Make process sleep until an event occurs on a file descriptor in
		// `fds`
		scheduler::end_tick();
	}
}
