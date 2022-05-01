//! The ioctl syscall allows to control a device represented by a file descriptor.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::regs::Regs;

// ioctl requests: TTY

/// ioctl request: Get the foreground process group ID on the terminal.
pub const TIOCGPGRP: u32 = 0x0000540f;
/// ioctl request: Set the foreground process group ID on the terminal.
pub const TIOCSPGRP: u32 = 0x00005410;
/// ioctl request: Returns the window size of the terminal.
pub const TIOCGWINSZ: u32 = 0x00005413;

/// The implementation of the `ioctl` syscall.
pub fn ioctl(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let request = regs.ecx as u32;
	let argp = regs.edx as *const c_void;

	crate::println!("ioctl: {} {:x} {:p}", fd, request, argp); // TODO rm

	// Getting the memory space and file
	let (mem_space, open_file_mutex) = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		(proc.get_mem_space().unwrap(), proc.get_open_file(fd as _).ok_or_else(|| errno!(EBADF))?)
	};

	// Getting the device file
	let mut open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

	// Executing ioctl with the current memory space
	let ret = open_file.ioctl(mem_space, request, argp)?;

	Ok(ret as _)
}
