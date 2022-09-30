//! This module implements the `write` system call, which allows to write data to a file.

use core::cmp::min;
use crate::errno::Errno;
use crate::errno;
use crate::file::open_file::O_NONBLOCK;
use crate::idt;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;
use crate::syscall::Signal;
use crate::util::io::IO;

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
		super::util::signal_check(regs);

		let (mem_space, open_file_mutex) = {
			let mutex = Process::get_current().unwrap();
			let guard = mutex.lock();
			let proc = guard.get_mut();

			let mem_space = proc.get_mem_space().unwrap();
			let open_file_mutex = proc.get_fd(fd).ok_or(errno!(EBADF))?.get_open_file();
			(mem_space, open_file_mutex)
		};

		// Trying to write and getting the length of written data
		let (len, flags) = idt::wrap_disable_interrupts(|| {
			let open_file_guard = open_file_mutex.lock();
			let open_file = open_file_guard.get_mut();

			let mem_space_guard = mem_space.lock();
			let buf_slice = buf.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

			let flags = open_file.get_flags();
			let len = match open_file.write(0, buf_slice) {
				Ok(len) => len,

				Err(e) => {
					// If writing to a broken pipe, kill with SIGPIPE
					if e.as_int() == errno::EPIPE {
						let mutex = Process::get_current().unwrap();
						let guard = mutex.lock();
						let proc = guard.get_mut();

						proc.kill(&Signal::SIGPIPE, false);
					}

					return Err(e);
				},
			};

			Ok((len, flags))
		})?;

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
