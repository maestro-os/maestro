//! The getcwd system call allows to retrieve the current working directory of
//! the current process.

use crate::errno;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `getcwd` syscall.
#[syscall]
pub fn getcwd(buf: SyscallSlice::<u8>, size: usize) -> Result<i32, Errno> {
	if size == 0 {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let cwd = proc.get_cwd().as_string()?;

	// Checking that the buffer is large enough
	if (size as usize) < cwd.len() + 1 {
		return Err(errno!(ERANGE));
	}

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let cwd_slice = cwd.as_bytes();
	let buf_slice = buf
		.get_mut(&mem_space_guard, size as _)?
		.ok_or_else(|| errno!(EINVAL))?;
	for i in 0..cwd.len() {
		buf_slice[i] = cwd_slice[i];
	}
	buf_slice[cwd.len()] = b'\0';

	Ok(buf.as_ptr() as _)
}
