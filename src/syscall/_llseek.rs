//! The `_llseek` system call repositions the offset of a file descriptor.

use core::ffi::c_uint;
use core::ffi::c_ulong;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::util::io::IO;
use macros::syscall;

/// Sets the offset from the given value.
const SEEK_SET: u32 = 0;
/// Sets the offset relative to the current offset.
const SEEK_CUR: u32 = 1;
/// Sets the offset relative to the end of the file.
const SEEK_END: u32 = 2;

/// The implementation of the `_llseek` syscall.
#[syscall]
pub fn _llseek(fd: c_uint, offset_high: c_ulong, offset_low: c_ulong, result: SyscallPtr::<u64>, whence: c_uint) -> Result<i32, Errno> {
	let (mem_space, open_file_mutex) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let open_file_mutex = proc
			.get_fd(fd)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file();

		(mem_space, open_file_mutex)
	};

	// Getting file
	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

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
		let mem_space_guard = mem_space.lock();
		// Writing the result to the userspace
		if let Some(result) = result.get_mut(&mem_space_guard)? {
			*result = off;
		}
	}

	// Setting the offset
	open_file.set_offset(off);

	Ok(0)
}
