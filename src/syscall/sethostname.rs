//! The `sethostname` syscall sets the hostname of the system.

use crate::errno::Errno;
use crate::file;
use crate::limits;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;
use crate::process::Process;

/// The implementation of the `sethostname` syscall.
pub fn sethostname(regs: &Regs) -> Result<i32, Errno> {
	let name: SyscallSlice<u8> = (regs.ebx as usize).into();
	let len = regs.ecx as usize;

	// Check the size of the hostname is in bounds
	if len > limits::HOST_NAME_MAX {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking permission
	if proc.get_euid() != file::ROOT_UID {
		return Err(errno!(EPERM));
	}

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let name_slice = name.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

	let hostname_guard = crate::HOSTNAME.lock();
	let hostname = hostname_guard.get_mut();
	hostname.resize(len)?;
	hostname.as_mut_slice().copy_from_slice(name_slice);

	Ok(0)
}
