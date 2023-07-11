//! The `sendto` system call sends a message on a socket.

use crate::errno::Errno;
use crate::file::buffer;
use crate::file::buffer::socket::Socket;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use core::any::Any;
use core::ffi::c_int;
use macros::syscall;

// TODO implement flags

#[syscall]
pub fn sendto(
	sockfd: c_int,
	buf: SyscallSlice<u8>,
	len: usize,
	_flags: c_int,
	dest_addr: SyscallSlice<u8>,
	addrlen: isize,
) -> Result<i32, Errno> {
	if sockfd < 0 {
		return Err(errno!(EBADF));
	}
	if addrlen < 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Get socket
	let fds_mutex = proc.get_fds().unwrap();
	let fds = fds_mutex.lock();
	let fd = fds.get_fd(sockfd as _).ok_or_else(|| errno!(EBADF))?;
	let open_file_mutex = fd.get_open_file()?;
	let open_file = open_file_mutex.lock();
	let sock_mutex = buffer::get(open_file.get_location()).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let _sock = (&mut *sock as &mut dyn Any)
		.downcast_mut::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;

	// Get slices
	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let _buf_slice = buf.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;
	let _dest_addr_slice = dest_addr
		.get(&mem_space_guard, addrlen as _)?
		.ok_or(errno!(EFAULT))?;

	// TODO
	todo!()
}
