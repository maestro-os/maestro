//! The writev system call allows to write sparse data on a file descriptor in on call.

use core::cmp::min;
use core::mem::size_of;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::limits;
use crate::process::Process;
use crate::process::Regs;
use crate::process::iovec::IOVec;

/// The implementation of the `writev` syscall.
pub fn writev(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let iov = regs.ecx as *const IOVec;
	let iovcnt = regs.edx as i32;

	// Checking the size of the vector is in bounds
	if iovcnt < 0 || iovcnt as usize > limits::IOV_MAX {
		return Err(errno::EINVAL);
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking that the vector is accessible
	if !proc.get_mem_space().unwrap().can_access(iov as *const _,
		iovcnt as usize * size_of::<IOVec>(), true, false) {
		return Err(errno::EFAULT);
	}

	let iov_slice = unsafe { // Safe because the access is checked before
		slice::from_raw_parts(iov, iovcnt as _)
	};

	// TODO Compute total length, then check it is in bound
	// Checking access to each buffers
	for i in iov_slice {
		if !proc.get_mem_space().unwrap().can_access(i.iov_base as *const _, i.iov_len, true,
			false) {
			return Err(errno::EFAULT);
		}
	}

	let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;

	let mut total_len = 0;

	for i in iov_slice {
		// The size to write. This is limited to avoid an overflow on the total length
		let l = min(i.iov_len, i32::MAX as usize - total_len);
		// The slice on the data
		let slice = unsafe { // Safe because the access is checked before
			slice::from_raw_parts(i.iov_base as *const u8, l)
		};

		total_len += fd.write(slice)?;
	}

	Ok(total_len as _)
}
