//! The `socketpair` system call creates a pair of file descriptor to an unnamed socket which can
//! be used for IPC (Inter-Process Communication).

use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::socket::Socket;
use crate::file::socket::SocketSide;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;

/// The implementation of the `socketpair` syscall.
pub fn socketpair(regs: &Regs) -> Result<i32, Errno> {
	let domain = regs.ebx as i32;
	let type_ = regs.ecx as i32;
	let protocol = regs.edx as i32;
	let sv: SyscallPtr<[i32; 2]> = (regs.esi as usize).into();

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let sv_slice = sv.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let sock = Socket::new(domain, type_, protocol)?;
	let sock2 = sock.clone();
	let (fd0_id, _) = proc.create_fd(file_descriptor::O_RDWR,
		FDTarget::Socket(SocketSide::new(sock, false)?))?;
	let (fd1_id, _) = proc.create_fd(file_descriptor::O_RDWR,
		FDTarget::Socket(SocketSide::new(sock2, true)?))?;

	sv_slice[0] = fd0_id as _;
	sv_slice[1] = fd1_id as _;
	Ok(0)
}
