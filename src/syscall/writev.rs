//! The `writev` system call allows to write sparse data on a file descriptor.

use crate::errno;
use crate::errno::Errno;
use crate::file::open_file::OpenFile;
use crate::file::open_file::O_NONBLOCK;
use crate::limits;
use crate::process::iovec::IOVec;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::mem_space::MemSpace;
use crate::process::scheduler;
use crate::process::signal::Signal;
use crate::process::Process;
use crate::util::io;
use crate::util::io::IO;
use core::cmp::min;
use core::ffi::c_int;
use macros::syscall;

// TODO Handle blocking writes (and thus, EINTR)

/// Writes the given chunks to the file.
///
/// Arguments:
/// - `mem_space` is the memory space of the current process.
/// - `iov` is the set of chunks.
/// - `open_file` is the file to write to.
fn write(mem_space: &MemSpace, iov: &[IOVec], open_file: &mut OpenFile) -> Result<i32, Errno> {
	let mut total_len = 0;

	for i in iov {
		// Ignoring zero entry
		if i.iov_len == 0 {
			continue;
		}

		// The size to write. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, i32::MAX as usize - total_len);
		let ptr = SyscallSlice::<u8>::from(i.iov_base as usize);

		if let Some(slice) = ptr.get(mem_space, l)? {
			total_len += open_file.write(0, slice)? as usize;
		}
	}

	Ok(total_len as _)
}

/// Peforms the writev operation.
///
/// Arguments:
/// - `fd` is the file descriptor.
/// - `iov` the IO vector.
/// - `iovcnt` the number of entries in the IO vector.
/// - `offset` is the offset in the file.
/// - `flags` is the set of flags.
pub fn do_writev(
	fd: i32,
	iov: SyscallSlice<IOVec>,
	iovcnt: i32,
	offset: Option<isize>,
	_flags: Option<i32>,
) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}
	if iovcnt < 0 || iovcnt as usize > limits::IOV_MAX {
		return Err(errno!(EINVAL));
	}

	let (proc, mem_space, open_file_mutex) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();
		let open_file_mutex = fds.get_fd(fd as _).ok_or(errno!(EBADF))?.get_open_file()?;

		drop(proc);
		(proc_mutex, mem_space, open_file_mutex)
	};

	let start_off = match offset {
		Some(o @ 0..) => o as u64,
		None | Some(-1) => {
			let open_file = open_file_mutex.lock();
			open_file.get_offset()
		}

		Some(..-1) => return Err(errno!(EINVAL)),
		// Required because of compiler bug
		Some(_) => unreachable!(),
	};

	loop {
		// TODO super::util::signal_check(regs);

		{
			let mem_space_guard = mem_space.lock();
			let iov_slice = iov
				.get(&mem_space_guard, iovcnt as _)?
				.ok_or(errno!(EFAULT))?;

			let mut open_file = open_file_mutex.lock();
			let flags = open_file.get_flags();

			// Change the offset temporarily
			let prev_off = open_file.get_offset();
			open_file.set_offset(start_off);

			let len = match write(&mem_space_guard, &iov_slice, &mut open_file) {
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

			// Restore previous offset
			open_file.set_offset(prev_off);

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

#[syscall]
pub fn writev(fd: c_int, iov: SyscallSlice<IOVec>, iovcnt: c_int) -> Result<i32, Errno> {
	do_writev(fd, iov, iovcnt, None, None)
}
