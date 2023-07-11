//! The `splice` system call splice data from one pipe to another.

use crate::errno::Errno;
use crate::file::FileType;
use crate::memory::malloc;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::util::io::IO;
use core::cmp::min;
use core::ffi::c_int;
use core::ffi::c_uint;
use macros::syscall;

#[syscall]
pub fn splice(
	fd_in: c_int,
	off_in: SyscallPtr<u64>,
	fd_out: c_int,
	off_out: SyscallPtr<u64>,
	len: usize,
	_flags: c_uint,
) -> Result<i32, Errno> {
	if fd_in < 0 || fd_out < 0 {
		return Err(errno!(EBADF));
	}

	let (input_mutex, off_in, output_mutex, off_out) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let input = fds
			.get_fd(fd_in as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()?;
		let output = fds
			.get_fd(fd_out as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()?;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let off_in = off_in.get(&mem_space_guard)?.cloned();
		let off_out = off_out.get(&mem_space_guard)?.cloned();

		(input, off_in, output, off_out)
	};

	{
		let input_type = input_mutex.lock().get_file()?.lock().get_type();
		let output_type = output_mutex.lock().get_file()?.lock().get_type();

		let in_is_pipe = matches!(input_type, FileType::Fifo);
		let out_is_pipe = matches!(output_type, FileType::Fifo);

		if !in_is_pipe && !out_is_pipe {
			return Err(errno!(EINVAL));
		}
		if in_is_pipe && off_in.is_some() {
			return Err(errno!(ESPIPE));
		}
		if out_is_pipe && off_out.is_some() {
			return Err(errno!(ESPIPE));
		}
	}

	let len = min(len, i32::MAX as usize);

	// TODO implement flags

	let mut buff = unsafe {
		// Safe because initialized memory is never read
		malloc::Alloc::<u8>::new(len)
	}?;

	let len = {
		let mut input = input_mutex.lock();

		let prev_off = input.get_offset();

		let (len, _) = input.read(off_in.unwrap_or(0), buff.as_slice_mut())?;

		if off_in.is_some() {
			input.set_offset(prev_off);
		}

		len
	};

	let in_slice = &buff.as_slice()[..(len as usize)];
	let mut i = 0;

	while i < len {
		// TODO Check for signal (and handle syscall restart correctly with offsets)

		let mut output = output_mutex.lock();

		let prev_off = output.get_offset();

		let l = output.write(off_out.unwrap_or(0), in_slice)?;

		if off_out.is_some() {
			output.set_offset(prev_off);
		}

		i += l;
	}

	Ok(len as _)
}
