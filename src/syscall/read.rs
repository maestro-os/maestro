//! The read system call allows to read the content of an open file.

use crate::errno;
use crate::errno::Errno;
use crate::file::open_file::O_NONBLOCK;
use crate::idt;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::scheduler;
use crate::process::Process;
use crate::util::io;
use crate::util::io::IO;
use core::cmp::min;
use core::ffi::c_int;
use macros::syscall;

// TODO O_ASYNC

#[syscall]
pub fn read(fd: c_int, buf: SyscallSlice<u8>, count: usize) -> Result<i32, Errno> {
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

		let open_file_mutex = fds.get_fd(fd as _).ok_or(errno!(EBADF))?.get_open_file()?;

		(mem_space, open_file_mutex)
	};

	loop {
		super::util::signal_check(regs);

		let (len, flags) = {
			let (len, eof, flags) = idt::wrap_disable_interrupts(|| {
				let mut open_file = open_file_mutex.lock();

				let mut mem_space_guard = mem_space.lock();
				let buf_slice = buf
					.get_mut(&mut mem_space_guard, len)?
					.ok_or(errno!(EFAULT))?;

				let flags = open_file.get_flags();
				let (len, eof) = open_file.read(0, buf_slice)?;
				Ok((len, eof, flags))
			})?;

			if len == 0 && eof {
				return Ok(0);
			}

			(len, flags)
		};

		if len > 0 || flags & O_NONBLOCK != 0 {
			// The file descriptor is non blocking
			return Ok(len as _);
		}

		// Make process sleep
		{
			let proc_mutex = Process::get_current().unwrap();
			let mut proc = proc_mutex.lock();

			let mut open_file = open_file_mutex.lock();
			open_file.add_waiting_process(&mut *proc, io::POLLIN | io::POLLERR)?;
		}
		unsafe {
			scheduler::end_tick();
		}
	}
}
