//! The `connect` system call connects a socket to a distant host.

use crate::errno::Errno;
use crate::file::open_file::FDTarget;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

/// The implementation of the `connect` syscall.
pub fn connect(regs: &Regs) -> Result<i32, Errno> {
	let sockfd = regs.ebx as i32;
	let addr: SyscallSlice<u8> = (regs.ecx as usize).into();
	let addrlen = regs.edx as usize;

	if sockfd < 0 {
		return Err(errno!(EBADF));
	}

	let proc_mutex = Process::get_current().unwrap();
	let proc_guard = proc_mutex.lock();
	let proc = proc_guard.get();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let addr_slice = addr.get(&mem_space_guard, addrlen)?.ok_or_else(|| errno!(EFAULT))?;

	let fd = proc.get_fd(sockfd as _).ok_or_else(|| errno!(EBADF))?;
	let open_file_mutex = fd.get_open_file();
	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get();

	match open_file.get_target() {
		FDTarget::File(_file_mutex) => {
			// TODO
			todo!();
		},

		FDTarget::Pipe(_) => return Err(errno!(ENOTSOCK)),

		FDTarget::Socket(sock_side_mutex) => {
			let sock_side_guard = sock_side_mutex.lock();
			let sock_side = sock_side_guard.get();

			let sock_mutex = sock_side.get_socket();
			let sock_guard = sock_mutex.lock();
			let sock = sock_guard.get_mut();

			sock.connect(addr_slice)?;
		},
	}

	Ok(0)
}
