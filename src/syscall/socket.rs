//! The `socket` system call allows to create a socket.

use crate::errno::Errno;
use crate::errno;
use crate::file::open_file::FDTarget;
use crate::file::open_file;
use crate::file::socket::SockDomain;
use crate::file::socket::SockType;
use crate::file::socket::Socket;
use crate::file::socket::SocketSide;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `socket` syscall.
pub fn socket(regs: &Regs) -> Result<i32, Errno> {
	let domain = regs.ebx as i32;
	let type_ = regs.ecx as i32;
	let protocol = regs.edx as i32;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let uid = proc.get_euid();
	let gid = proc.get_egid();

	let sock_domain = SockDomain::from(domain).ok_or_else(|| errno!(EAFNOSUPPORT))?;
	let sock_type = SockType::from(type_).ok_or_else(|| errno!(EPROTONOSUPPORT))?;
	if !sock_domain.can_use(uid, gid) || !sock_type.can_use(uid, gid) {
		return Err(errno!(EACCES));
	}

	let sock = Socket::new(sock_domain, sock_type, protocol)?;
	let sock_fd = proc.create_fd(
		open_file::O_RDWR,
		FDTarget::Socket(SocketSide::new(sock, false)?),
	)?;

	Ok(sock_fd.get_id() as _)
}
