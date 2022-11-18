//! The `writev` system call allows to write sparse data on a file descriptor.

use core::ffi::c_int;
use crate::errno;
use crate::errno::Errno;
use crate::file::open_file::OpenFile;
use crate::idt;
use crate::limits;
use crate::process::iovec::IOVec;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::mem_space::MemSpace;
use crate::process::signal::Signal;
use crate::process::Process;
use crate::util::io::IO;
use crate::util::ptr::IntSharedPtr;
use core::cmp::min;
use macros::syscall;

// TODO Handle blocking writes (and thus, EINTR)

// TODO Check the operation is atomic on the file?
/// Writes the given chunks of files to the file.
/// `mem_space` is the memory space of the current process.
/// `iov` is the set of chunks.
/// `iovcnt` is the number of chunks in `iov`.
/// `open_file` is the file to write to.
fn write(
	mem_space: IntSharedPtr<MemSpace>,
	iov: SyscallSlice<IOVec>,
	iovcnt: usize,
	open_file: &mut OpenFile,
) -> Result<i32, Errno> {
	let mem_space_guard = mem_space.lock();
	let iov_slice = iov.get(&mem_space_guard, iovcnt)?.ok_or(errno!(EFAULT))?;

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
			// TODO Handle in a loop like `write`?
			total_len += open_file.write(0, slice)? as usize;
		}
	}

	Ok(total_len as _)
}

/// Peforms the writev operation.
/// TODO doc params
pub fn do_writev(
	fd: i32,
	iov: SyscallSlice<IOVec>,
	iovcnt: i32,
	offset: Option<isize>,
	_flags: Option<i32>,
) -> Result<i32, Errno> {
	// TODO Handle flags

	// Checking the size of the vector is in bounds
	if iovcnt < 0 || iovcnt as usize > limits::IOV_MAX {
		return Err(errno!(EINVAL));
	}

	let (mem_space, open_file_mutex) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let open_file_mutex = proc.get_fd(fd as _).ok_or(errno!(EBADF))?.get_open_file();
		(mem_space, open_file_mutex)
	};

	idt::wrap_disable_interrupts(|| {
		let open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get_mut();

		// The offset to restore on the fd after the write operation
		let mut prev_off = None;
		// Setting the offset temporarily
		if let Some(offset) = offset {
			if offset < -1 {
				return Err(errno!(EINVAL));
			}

			if offset != -1 {
				prev_off = Some(open_file.get_offset());
				open_file.set_offset(offset as _);
			}
		}

		let result = write(mem_space, iov, iovcnt as _, open_file);
		match &result {
			// If writing to a broken pipe, kill with SIGPIPE
			Err(e) if e.as_int() == errno::EPIPE => {
				let mutex = Process::get_current().unwrap();
				let guard = mutex.lock();
				let proc = guard.get_mut();

				proc.kill(&Signal::SIGPIPE, false);
			}

			_ => {}
		}

		// Restoring previous offset
		if let Some(prev_off) = prev_off {
			open_file.set_offset(prev_off);
		}

		result
	})
}

/// The implementation of the `writev` syscall.
#[syscall]
pub fn writev(fd: c_int, iov: SyscallSlice<IOVec>, iovcnt: c_int) -> Result<i32, Errno> {
	do_writev(fd, iov, iovcnt, None, None)
}
