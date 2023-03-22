//! The `connect` system call connects a socket to a distant host.

use core::any::Any;
use core::ffi::c_int;
use crate::errno::Errno;
use crate::file::buffer::socket::SockState;
use crate::file::buffer::socket::Socket;
use crate::file::buffer;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use macros::syscall;

/// The implementation of the `connect` syscall.
#[syscall]
pub fn connect(sockfd: c_int, addr: SyscallSlice<u8>, addrlen: usize) -> Result<i32, Errno> {
	if sockfd < 0 {
		return Err(errno!(EBADF));
	}

	let proc_mutex = Process::get_current().unwrap();
	let proc = proc_mutex.lock();

	let mem_space_mutex = proc.get_mem_space().unwrap();
	let mem_space = mem_space_mutex.lock();
	let addr_slice = addr.get(&mem_space, addrlen)?.ok_or_else(|| errno!(EFAULT))?;

	let fds_mutex = proc.get_fds().unwrap();
	let fds = fds_mutex.lock();
	let fd = fds.get_fd(sockfd as _).ok_or_else(|| errno!(EBADF))?;

	let open_file_mutex = fd.get_open_file()?;
	let open_file = open_file_mutex.lock();

	let sock_mutex = buffer::get(open_file.get_location()).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let sock = (&mut *sock as &mut dyn Any).downcast_mut::<Socket>().unwrap();

	sock.connect(addr_slice)?;

	// Waiting until the socket turns into Ready state
	while !matches!(sock.get_state(), SockState::Ready) {
		// Checking for pending signal
		super::util::signal_check(regs);
		// NOTE: If the syscall resumes, it must not re-call the `connect` function

		// TODO Make the process sleep
	}

	Ok(0)
}
