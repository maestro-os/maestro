//! The `poll` system call allows to wait for events on a given set of file descriptors.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;
use crate::time::unit::Timestamp;
use crate::time;

/// There is data to read.
const POLLIN: i16     = 0b0000000001;
/// There is some exceptional condition on the file descriptor.
const POLLPRI: i16    = 0b0000000010;
/// Writing is now possible.
const POLLOUT: i16    = 0b0000000100;
/// Error condition.
const POLLERR: i16    = 0b0000001000;
/// Hang up.
const POLLHUP: i16    = 0b0000010000;
/// Invalid request: fd not open.
const POLLNVAL: i16   = 0b0000100000;
/// Equivalent to POLLIN.
const POLLRDNORM: i16 = 0b0001000000;
/// Priority band data can be read.
const POLLRDBAND: i16 = 0b0010000000;
/// Equivalent to POLLOUT.
const POLLWRNORM: i16 = 0b0100000000;
/// Priority data may be written.
const POLLWRBAND: i16 = 0b1000000000;

/// Structure representing a file descriptor passed to the `poll` system call.
#[repr(C)]
struct PollFD {
	/// The file descriptor.
	fd: i32,
	/// The input mask telling which events to look for.
	events: i16,
	/// The output mask telling which events happened.
	revents: i16,
}

/// The implementation of the `poll` syscall.
pub fn poll(regs: &Regs) -> Result<i32, Errno> {
	let fds: SyscallSlice<PollFD> = (regs.ebx as usize).into();
	let nfds = regs.ecx as usize;
	let timeout = regs.edx as i32;

	// The timeout. None means no timeout
	let to: Option<Timestamp> = if timeout >= 0 {
		Some(timeout as _)
	} else {
		None
	};

	// The start timestamp
	let start_ts = time::get().unwrap_or(0);

	loop {
		// Checking whether the system call timed out
		if let Some(timeout) = to {
			if time::get().unwrap_or(0) >= start_ts + timeout {
				return Ok(0);
			}
		}

		{
			let mutex = Process::get_current().unwrap();
			let guard = mutex.lock();
			let proc = guard.get_mut();

			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();

			let fds = fds.get(&mem_space_guard, nfds)?.ok_or_else(|| errno!(EFAULT))?;

			// Checking the file descriptors list
			for fd in fds {
				// TODO Handle POLLERR, POLLHUP and POLLNVAL

				if fd.events & POLLIN != 0 {
					// TODO
					todo!();
				}

				if fd.events & POLLPRI != 0 {
					// TODO
					todo!();
				}

				if fd.events & POLLOUT != 0 {
					// TODO
					todo!();
				}

				if fd.events & POLLRDNORM != 0 {
					// TODO
					todo!();
				}

				if fd.events & POLLRDBAND != 0 {
					// TODO
					todo!();
				}

				if fd.events & POLLWRNORM != 0 {
					// TODO
					todo!();
				}

				if fd.events & POLLWRBAND != 0 {
					// TODO
					todo!();
				}
			}

			// The number of file descriptor with at least one event
			let fd_event_count = fds.iter().filter(| fd | fd.revents != 0).count();
			// If at least on event happened, return the number of file descriptors concerned
			if fd_event_count > 0 {
				return Ok(fd_event_count as _);
			}
		}

		// TODO Make process Sleeping until an event happens on a file descriptor in `fds`
		crate::wait();
	}
}
