//! The `poll` system call allows to wait for events on a given set of file descriptors.

use core::slice;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::Regs;
use crate::time::Timestamp;
use crate::time;

/// There is data to read.
const POLLIN: i16 = 0b0; // TODO Put the correct value
/// There is some exceptional condition on the file descriptor.
const POLLPRI: i16 = 0b0; // TODO Put the correct value
/// Writing is now possible.
const POLLOUT: i16 = 0b0; // TODO Put the correct value
/// Stream socket peer closed connection, or shut down writing half of connection.
const POLLRDHUP: i16 = 0b0; // TODO Put the correct value
/// Error condition.
const POLLERR: i16 = 0b0; // TODO Put the correct value
/// Hang up.
const POLLHUP: i16 = 0b0; // TODO Put the correct value
/// Invalid request: fd not open.
const POLLNVAL: i16 = 0b0; // TODO Put the correct value
/// Equivalent to POLLIN.
const POLLRDNORM: i16 = 0b0; // TODO Put the correct value
/// Priority band data can be read.
const POLLRDBAND: i16 = 0b0; // TODO Put the correct value
/// Equivalent to POLLOUT.
const POLLWRNORM: i16 = 0b0; // TODO Put the correct value
/// Priority data may be written.
const POLLWRBAND: i16 = 0b0; // TODO Put the correct value

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
	let fds = regs.ebx as *mut PollFD;
	let nfds = regs.ecx as usize;
	let timeout = regs.edx as i32;

	// The timeout. None means no timeout
	let to: Option<Timestamp> = if timeout >= 0 {
		Some(timeout as _)
	} else {
		None
	};

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let _proc = guard.get_mut();

	// TODO Check access to `fds`

	let fds = unsafe { // Safe because access has been checked before
		slice::from_raw_parts(fds, nfds)
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

		// Checking the file descriptors list
		for _fd in fds {
			// TODO
			todo!();
		}

		// TODO Make process Sleeping until an event happens on a file descriptor in `fds`
		crate::wait();
	}
}
