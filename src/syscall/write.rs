//! This module implements the `write` system call, which allows to write data to a file.

use core::cmp::min;
use crate::errno::Errno;
use crate::errno;
use crate::file::open_file::O_NONBLOCK;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;
use crate::process::signal;
use crate::syscall::Signal;

// TODO O_ASYNC

/// The implementation of the `write` syscall.
pub fn write(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf: SyscallSlice<u8> = (regs.ecx as usize).into();
	let count = regs.edx as usize;

	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}

	loop {
		// Trying to write and getting the length of written data
		let (len, flags) = {
			let (mem_space, open_file_mutex) = {
				let mutex = Process::get_current().unwrap();
				let mut guard = mutex.lock();
				let proc = guard.get_mut();

				(proc.get_mem_space().unwrap(), proc.get_fd(fd).ok_or(errno!(EBADF))?.get_open_file())
			};

			let mem_space_guard = mem_space.lock();
			let buf_slice = buf.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

			let mut open_file_guard = open_file_mutex.lock();
			let open_file = open_file_guard.get_mut();

			let flags = open_file.get_flags();
			let len = match open_file.write(buf_slice) {
				Ok(len) => len,

				Err(err) => {
					// If the pipe is broken, kill with SIGPIPE
					if err.as_int() == errno::EPIPE {
						let mutex = Process::get_current().unwrap();
						let mut guard = mutex.lock();
						let proc = guard.get_mut();

						proc.kill(Signal::new(signal::SIGPIPE).unwrap(), false);
					}

					return Err(err);
				},
			};

			(len, flags)
		};

		// TODO Continue until everything was written?
		// If the length is greater than zero, success
		if len > 0 {
			return Ok(len as _);
		}

		if flags & O_NONBLOCK != 0 {
			// The file descriptor is non blocking
			return Err(errno!(EAGAIN));
		}

		// TODO Mark the process as Sleeping and wake it up when data can be written?
		crate::wait();
	}
}
