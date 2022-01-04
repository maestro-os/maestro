//! The `socketpair` system call creates a pair of file descriptor to an unnamed socket which can
//! be used for IPC (Inter-Process Communication).

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::socket::Socket;
use crate::file::socket::SocketSide;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `socketpair` syscall.
pub fn socketpair(regs: &Regs) -> Result<i32, Errno> {
	let domain = regs.ebx as i32;
	let type_ = regs.ecx as i32;
	let protocol = regs.edx as i32;
	let sv = regs.esi as *mut [i32; 2];

	let (fd0, fd1) = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let len = size_of::<[i32; 2]>();
		if !proc.get_mem_space().unwrap().can_access(sv as _, len, true, true) {
			return Err(errno::EFAULT);
		}

		let sock = Socket::new(domain, type_, protocol)?;
		let sock2 = sock.clone();
		let fd0 = proc.create_fd(file_descriptor::O_RDWR,
			FDTarget::Socket(SocketSide::new(sock, false)?))?.get_id();
		let fd1 = proc.create_fd(file_descriptor::O_RDWR,
			FDTarget::Socket(SocketSide::new(sock2, true)?))?.get_id();

		(fd0, fd1)
	};

	unsafe { // Safe because the address has been check before
		(*sv)[0] = fd0 as _;
		(*sv)[1] = fd1 as _;
	}
	Ok(0)
}
