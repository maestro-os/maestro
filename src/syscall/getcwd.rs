//! The getcwd system call allows to retrieve the current working directory of
//! the current process.

use crate::errno;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use crate::util;
use macros::syscall;

#[syscall]
pub fn getcwd(buf: SyscallSlice<u8>, size: usize) -> Result<i32, Errno> {
	if size == 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let cwd = crate::format!("{}", proc.get_cwd())?;

	// Checking that the buffer is large enough
	if size < cwd.len() + 1 {
		return Err(errno!(ERANGE));
	}

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	let cwd_slice = cwd.as_bytes();
	let buf_slice = buf
		.get_mut(&mut mem_space_guard, size as _)?
		.ok_or_else(|| errno!(EINVAL))?;
	util::slice_copy(cwd_slice, buf_slice);
	buf_slice[cwd.len()] = b'\0';

	Ok(buf.as_ptr() as _)
}
