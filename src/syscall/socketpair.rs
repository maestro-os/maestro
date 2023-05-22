//! The `socketpair` system call creates a pair of file descriptor to an unnamed
//! socket which can be used for IPC (Inter-Process Communication).

use crate::errno;
use crate::errno::Errno;
use crate::file::buffer;
use crate::file::buffer::socket::Socket;
use crate::file::open_file;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn socketpair(
	domain: c_int,
	r#type: c_int,
	protocol: c_int,
	sv: SyscallPtr<[c_int; 2]>,
) -> Result<i32, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();
	let sv_slice = sv.get_mut(&mut mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let fds_mutex = proc.get_fds().unwrap();
	let mut fds = fds_mutex.lock();

	// Create socket
	let sock = Socket::new(domain, r#type, protocol);
	let loc = buffer::register(None, Arc::new(Mutex::new(sock))?)?;
	open_file::OpenFile::new(loc.clone(), open_file::O_RDWR)?;

	let fd0 = fds.create_fd(loc.clone(), 0, true, true)?;
	sv_slice[0] = fd0.get_id() as _;

	let fd1 = fds.create_fd(loc, 0, true, true)?;
	sv_slice[1] = fd1.get_id() as _;

	Ok(0)
}
