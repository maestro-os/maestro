//! The writev system call allows to write sparse data on a file descriptor in on call.

use core::cmp::min;
use crate::errno::Errno;
use crate::errno;
use crate::limits;
use crate::process::Process;
use crate::process::iovec::IOVec;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

/// The implementation of the `writev` syscall.
pub fn writev(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let iov: SyscallSlice<IOVec> = (regs.ecx as usize).into();
	let iovcnt = regs.edx as i32;

	// Checking the size of the vector is in bounds
	if iovcnt < 0 || iovcnt as usize > limits::IOV_MAX {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let iov_slice = iov.get(&mem_space_guard, iovcnt as _)?.ok_or(errno!(EFAULT))?;

	let fd = proc.get_fd(fd).ok_or(errno!(EBADF))?;

	// TODO If total length gets out of bounds, stop
	let mut total_len = 0;

	for i in iov_slice {
		// Ignoring zero entry
		if i.iov_len == 0 {
			continue;
		}

		// The size to write. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, i32::MAX as usize - total_len);
		let ptr = SyscallSlice::<u8>::from(i.iov_base as usize);

		if let Some(slice) = ptr.get(&mem_space_guard, l)? {
			total_len += fd.write(slice)?;
		}
	}

	Ok(total_len as _)
}
