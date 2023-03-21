//! This module implements the `write` system call, which allows to write data
//! to a file.

use core::cmp::min;
use core::ffi::c_int;
use crate::errno::Errno;
use crate::errno;
use crate::file::open_file::O_NONBLOCK;
use crate::idt;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::scheduler;
use crate::syscall::Signal;
use crate::util::io::IO;
use crate::util::io;
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

	let (mem_space, open_file_mutex) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let open_file_mutex = fds.get_fd(fd as _)
			.ok_or(errno!(EBADF))?
			.get_open_file()?;

		(mem_space, open_file_mutex)
	};

	loop {
		super::util::signal_check(regs);

		// Trying to write and getting the length of written data
		let (len, flags) = idt::wrap_disable_interrupts(|| {
			let mut open_file = open_file_mutex.lock();

			let mem_space_guard = mem_space.lock();
			let buf_slice = buf.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

			let flags = open_file.get_flags();
			let len = match open_file.write(0, buf_slice) {
				Ok(len) => len,

				Err(e) => {
					// If writing to a broken pipe, kill with SIGPIPE
					if e.as_int() == errno::EPIPE {
						let proc_mutex = Process::get_current().unwrap();
						let mut proc = proc_mutex.lock();

						proc.kill(&Signal::SIGPIPE, false);
					}

					return Err(e);
				}
			};

			Ok((len, flags))
		})?;

		// If the length is greater than zero, success
		if len > 0 {
			return Ok(len as _);
		}

		if flags & O_NONBLOCK != 0 {
			// The file descriptor is non blocking
			return Err(errno!(EAGAIN));
		}

		// Make process sleep
		{
			let proc_mutex = Process::get_current().unwrap();
			let mut proc = proc_mutex.lock();

			let mut open_file = open_file_mutex.lock();
			open_file.add_waiting_process(&mut *proc, io::POLLOUT | io::POLLERR)?;
		}
		unsafe {
			scheduler::end_tick();
		}
	}
}
