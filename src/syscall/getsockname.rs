//! The `getsockname` system call returns the socket address bound to a socket.

use crate::errno::Errno;
use crate::file::buffer;
use crate::file::buffer::socket::Socket;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use core::any::Any;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn getsockname(
	sockfd: c_int,
	addr: SyscallSlice<u8>,
	addrlen: SyscallPtr<isize>,
) -> Result<i32, Errno> {
	if sockfd < 0 {
		return Err(errno!(EBADF));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Get socket
	let fds_mutex = proc.get_fds().unwrap();
	let fds = fds_mutex.lock();
	let fd = fds.get_fd(sockfd as _).ok_or_else(|| errno!(EBADF))?;
	let open_file_mutex = fd.get_open_file()?;
	let open_file = open_file_mutex.lock();
	let loc = open_file.get_location();
	let sock_mutex = buffer::get(loc).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let sock = (&mut *sock as &mut dyn Any)
		.downcast_mut::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	// Read and check buffer length
	let addrlen_val = addrlen
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;
	if *addrlen_val < 0 {
		return Err(errno!(EINVAL));
	}
	let addrlen_val = *addrlen_val as usize;

	// Read socket name
	let addr_slice = addr
		.get_mut(&mut mem_space_guard, addrlen_val)?
		.ok_or(errno!(EFAULT))?;
	let len = sock.read_sockname(addr_slice) as _;
	drop(addr_slice);

	// Update actual length of the address
	let addrlen_val = addrlen
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;
	*addrlen_val = len;

	Ok(0)
}
