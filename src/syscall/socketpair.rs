//! The `socketpair` system call creates a pair of file descriptor to an unnamed
//! socket which can be used for IPC (Inter-Process Communication).

use crate::errno;
use crate::errno::Errno;
use crate::file::buffer;
use crate::file::buffer::socket::Socket;
use crate::file::open_file;
use crate::net::SocketDesc;
use crate::net::SocketDomain;
use crate::net::SocketType;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn socketpair(
	domain: c_int,
	r#type: c_int,
	protocol: c_int,
	sv: SyscallPtr<[c_int; 2]>,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let uid = proc.euid;
	let gid = proc.egid;

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();
	let sv_slice = sv.get_mut(&mut mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let sock_domain = SocketDomain::try_from(domain as u32)?;
	let sock_type = SocketType::try_from(r#type as u32)?;
	if !sock_domain.can_use(uid, gid) || !sock_type.can_use(uid, gid) {
		return Err(errno!(EACCES));
	}
	let desc = SocketDesc {
		domain: sock_domain,
		type_: sock_type,
		protocol,
	};

	let sock = Socket::new(desc)?;

	let loc = buffer::register(None, sock)?;
	open_file::OpenFile::new(loc.clone(), open_file::O_RDWR)?;

	let fds_mutex = proc.get_fds().unwrap();
	let mut fds = fds_mutex.lock();

	let fd0 = fds.create_fd(loc.clone(), 0, true, true)?;
	sv_slice[0] = fd0.get_id() as _;

	let fd1 = fds.create_fd(loc, 0, true, true)?;
	sv_slice[1] = fd1.get_id() as _;

	Ok(0)
}
