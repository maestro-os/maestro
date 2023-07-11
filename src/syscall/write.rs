//! This module implements the `write` system call, which allows to write data
//! to a file.

use crate::errno;
use crate::errno::Errno;
use crate::file::open_file::O_NONBLOCK;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::scheduler;
use crate::process::Process;
use crate::syscall::Signal;
use crate::util::io;
use crate::util::io::IO;
use core::cmp::min;
use core::ffi::c_int;
use macros::syscall;

// TODO O_ASYNC

#[syscall]
pub fn write(fd: c_int, buf: SyscallSlice<u8>, count: usize) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}

	let (proc, mem_space, open_file) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();
		let open_file_mutex = fds.get_fd(fd as _).ok_or(errno!(EBADF))?.get_open_file()?;

		drop(proc);
		(proc_mutex, mem_space, open_file_mutex)
	};

	loop {
		super::util::signal_check(regs);

		{
			let mem_space_guard = mem_space.lock();
			let buf_slice = buf.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

			// Write file
			let mut open_file = open_file.lock();
			let flags = open_file.get_flags();
			let len = match open_file.write(0, buf_slice) {
				Ok(len) => len,

				Err(e) => {
					// If writing to a broken pipe, kill with SIGPIPE
					if e.as_int() == errno::EPIPE {
						let mut proc = proc.lock();
						proc.kill(&Signal::SIGPIPE, false);
					}

					return Err(e);
				}
			};

			if len > 0 {
				return Ok(len as _);
			}
			if flags & O_NONBLOCK != 0 {
				// The file descriptor is non blocking
				return Err(errno!(EAGAIN));
			}

			// Block on file
			let mut proc = proc.lock();
			open_file.add_waiting_process(&mut proc, io::POLLOUT | io::POLLERR)?;
		}

		// Make current process sleep
		scheduler::end_tick();
	}
}
