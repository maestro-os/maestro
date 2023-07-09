//! The `_llseek` system call repositions the offset of a file descriptor.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::util::io::IO;
use core::ffi::c_uint;
use core::ffi::c_ulong;
use macros::syscall;

/// Sets the offset from the given value.
const SEEK_SET: u32 = 0;
/// Sets the offset relative to the current offset.
const SEEK_CUR: u32 = 1;
/// Sets the offset relative to the end of the file.
const SEEK_END: u32 = 2;

#[syscall]
pub fn _llseek(
	fd: c_uint,
	offset_high: c_ulong,
	offset_low: c_ulong,
	result: SyscallPtr<u64>,
	whence: c_uint,
) -> Result<i32, Errno> {
	let (mem_space, open_file_mutex) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let open_file_mutex = fds
			.get_fd(fd)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()?;

		(mem_space, open_file_mutex)
	};

	// Getting file
	let mut open_file = open_file_mutex.lock();

	// Computing the offset
	let off = ((offset_high as u64) << 32) | (offset_low as u64);
	let off = match whence {
		SEEK_SET => off,
		SEEK_CUR => open_file
			.get_offset()
			.checked_add(off)
			.ok_or_else(|| errno!(EOVERFLOW))?,
		SEEK_END => open_file
			.get_size()
			.checked_add(off)
			.ok_or_else(|| errno!(EOVERFLOW))?,

		_ => return Err(errno!(EINVAL)),
	};

	{
		let mut mem_space_guard = mem_space.lock();
		// Writing the result to the userspace
		if let Some(result) = result.get_mut(&mut mem_space_guard)? {
			*result = off;
		}
	}

	// Setting the offset
	open_file.set_offset(off);

	Ok(0)
}
