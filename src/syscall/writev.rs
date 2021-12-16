//! TODO doc

use core::cmp::min;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process::iovec::IOVec;

/// The implementation of the `writev` syscall.
pub fn writev(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let iov = regs.ecx as *const IOVec;
	let iovcnt = regs.edx as i32;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	// TODO Check access for each

	let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;

	// The total written length
	let mut total_len = 0;
	for i in 0..iovcnt {
		// The IO vector
		let iov = unsafe { // Safe because the access is checked before
			&*iov.add(i as _)
		};
		// The size to write. This is limited to avoid an overflow on the total length
		let l = min(iov.iov_len, i32::MAX as usize - total_len);
		// The slice on the data
		let slice = unsafe { // Safe because the access is checked before
			slice::from_raw_parts(iov.iov_base as *const u8, l)
		};

		total_len += fd.write(slice)?;
	}

	Ok(total_len as _)
}
