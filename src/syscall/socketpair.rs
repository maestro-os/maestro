//! The `socketpair` system call creates a pair of file descriptor to an unnamed
//! socket which can be used for IPC (Inter-Process Communication).

use core::ffi::c_int;
use crate::errno::Errno;
use crate::errno;
use crate::file::buffer::socket::Socket;
use crate::file::buffer;
use crate::file::open_file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::util::ptr::SharedPtr;
use macros::syscall;

/// The implementation of the `socketpair` syscall.
#[syscall]
pub fn socketpair(
	domain: c_int,
	r#type: c_int,
	protocol: c_int,
	sv: SyscallPtr<[c_int; 2]>,
) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let sv_slice = sv.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let fds_mutex = proc.get_fds().unwrap();
	let fds_guard = fds_mutex.lock();
	let fds = fds_guard.get_mut();

	// Create socket
	let sock = Socket::new(domain, r#type, protocol);
	let loc = buffer::register(None, SharedPtr::new(sock)?)?;
	open_file::OpenFile::new(loc.clone(), open_file::O_RDWR)?;

	let fd0 = fds.create_fd(loc.clone(), 0, true, true)?;
	sv_slice[0] = fd0.get_id() as _;

	let fd1 = fds.create_fd(loc, 0, true, true)?;
	sv_slice[1] = fd1.get_id() as _;

	Ok(0)
}
