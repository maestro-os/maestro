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

	let (mem_space, open_file_mutex) = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		(proc.get_mem_space().unwrap(), proc.get_open_file(fd).ok_or(errno!(EBADF))?)
	};

	let mem_space_guard = mem_space.lock();

	let iov_slice = iov.get(&mem_space_guard, iovcnt as _)?.ok_or(errno!(EFAULT))?;

	let mut open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

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
			total_len += open_file.write(slice)?;
		}
	}

	Ok(total_len as _)
}
