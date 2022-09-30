//! The `connect` system call connects a socket to a distant host.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::open_file::FDTarget;
use crate::file::socket::SockState;
use crate::file::vfs;
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

	let uid = proc.get_euid();
	let gid = proc.get_egid();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let addr_slice = addr.get(&mem_space_guard, addrlen)?.ok_or_else(|| errno!(EFAULT))?;

	let fd = proc.get_fd(sockfd as _).ok_or_else(|| errno!(EBADF))?;
	let open_file_mutex = fd.get_open_file();
	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get();

	let sock_mutex = match open_file.get_target() {
		FDTarget::File(file_mutex) => {
			let file_guard = file_mutex.lock();
			let file = file_guard.get_mut();

			match file.get_content() {
				FileContent::Socket => {
					if !file.can_write(uid, gid) {
						return Err(errno!(EACCES));
					}

					let vfs_mutex = vfs::get();
					let vfs_guard = vfs_mutex.lock();
					let vfs = vfs_guard.get_mut().as_mut().unwrap();

					vfs.get_named_socket(file.get_location())?
				},

				_ => return Err(errno!(ENOTSOCK)),
			}
		},

		FDTarget::Pipe(_) => return Err(errno!(ENOTSOCK)),

		FDTarget::Socket(sock_side_mutex) => {
			let sock_side_guard = sock_side_mutex.lock();
			let sock_side = sock_side_guard.get();

			sock_side.get_socket()
		},
	};

	let sock_guard = sock_mutex.lock();
	let sock = sock_guard.get_mut();

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
