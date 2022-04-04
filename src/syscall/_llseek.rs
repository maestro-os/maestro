//! The `_llseek` system call repositions the offset of a file descriptor.

use core::mem::size_of;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// Sets the offset from the given value.
const SEEK_SET: u32 = 0;
/// Sets the offset relative to the current offset.
const SEEK_CUR: u32 = 1;
/// Sets the offset relative to the end of the file.
const SEEK_END: u32 = 2;

/// The implementation of the `_llseek` syscall.
pub fn _llseek(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as u32;
	let offset_high = regs.ecx as u32;
	let offset_low = regs.edx as u32;
	let result = regs.esi as *mut u64;
	let whence = regs.edi as u32;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking access
	if !result.is_null()
		&& !proc.get_mem_space().unwrap().can_access(result as _, size_of::<u64>(), true, true) {
		return Err(errno!(EFAULT));
	}

	// Getting the file descriptor
	let fd = proc.get_fd(fd).ok_or(errno!(EBADF))?;

	// Computing the offset
	let off = ((offset_high as u64) << 32) | (offset_low as u64);
	let off = match whence {
		SEEK_SET => off,
		SEEK_CUR => fd.get_offset() + off,
		SEEK_END => fd.get_file_size() + off,

		_ => return Err(errno!(EINVAL)),
	};

	// Setting the offset
	fd.set_offset(off);

	// Writting the result to the userspace
	if !result.is_null() {
		unsafe { // Safe because access is checked before
			*result = off;
		}
	}

	Ok(0)
}
