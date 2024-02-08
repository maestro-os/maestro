//! The `sethostname` syscall sets the hostname of the system.

use crate::{
	errno::Errno,
	limits,
	process::{mem_space::ptr::SyscallSlice, Process},
	util::container::vec::Vec,
};
use macros::syscall;

#[syscall]
pub fn sethostname(name: SyscallSlice<u8>, len: usize) -> Result<i32, Errno> {
	// Check the size of the hostname is in bounds
	if len > limits::HOST_NAME_MAX {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Checking permission
	if !proc.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let name_slice = name.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

	let mut hostname = crate::HOSTNAME.lock();
	*hostname = Vec::from_slice(name_slice)?;

	Ok(0)
}
