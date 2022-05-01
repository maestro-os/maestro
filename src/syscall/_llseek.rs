//! The `_llseek` system call repositions the offset of a file descriptor.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
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
	let result: SyscallPtr::<u64> = (regs.esi as usize).into();
	let whence = regs.edi as u32;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();

	// Getting the file descriptor
	let open_file_mutex = proc.get_fd(fd).ok_or_else(|| errno!(EBADF))?.get_open_file();
	let mut open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

	// Computing the offset
	let off = ((offset_high as u64) << 32) | (offset_low as u64);
	let off = match whence {
		SEEK_SET => off,
		SEEK_CUR => open_file.get_offset() + off,
		SEEK_END => open_file.get_file_size() + off,

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
