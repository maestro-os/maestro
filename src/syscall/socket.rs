//! The `socket` system call allows to create a socket.

use crate::errno;
use crate::errno::Errno;
use crate::file::buffer;
use crate::file::buffer::socket::SockDomain;
use crate::file::buffer::socket::SockType;
use crate::file::buffer::socket::Socket;
use crate::file::open_file;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `socket` syscall.
#[syscall]
pub fn socket(domain: c_int, r#type: c_int, protocol: c_int) -> Result<i32, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let proc = proc_mutex.lock();

	let uid = proc.euid;
	let gid = proc.egid;

	let sock_domain = SockDomain::try_from(domain as u32)?;
	let sock_type = SockType::try_from(r#type as u32)?;
	if !sock_domain.can_use(uid, gid) || !sock_type.can_use(uid, gid) {
		return Err(errno!(EACCES));
	}

	let sock = Socket::new(sock_domain, sock_type, protocol)?;

	let loc = buffer::register(None, sock)?;
	open_file::OpenFile::new(loc.clone(), open_file::O_RDWR)?;

	let fds_mutex = proc.get_fds().unwrap();
	let mut fds = fds_mutex.lock();

	let sock_fd = fds.create_fd(loc, 0, true, true)?;

	Ok(sock_fd.get_id() as _)
}
