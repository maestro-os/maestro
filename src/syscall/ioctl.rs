//! The ioctl syscall allows to control a device represented by a file
//! descriptor.

use crate::errno;
use crate::errno::Errno;
use crate::process::regs::Regs;
use crate::process::Process;
use core::ffi::c_void;

// ioctl requests: TTY

/// ioctl request: Returns the current serial port settings.
pub const TCGETS: u32 = 0x00005401;
/// ioctl request: Sets the serial port settings. Making the change immediately.
pub const TCSETS: u32 = 0x00005402;
/// ioctl request: Sets the serial port settings. Making the change only when
/// all currently written data has been transmitted. At this points, any
/// received data is discarded.
pub const TCSETSW: u32 = 0x00005403;
/// ioctl request: Sets the serial port settings. Making the change only when
/// all currently written data has been transmitted.
pub const TCSETSF: u32 = 0x00005404;
/// ioctl request: Get the foreground process group ID on the terminal.
pub const TIOCGPGRP: u32 = 0x0000540f;
/// ioctl request: Set the foreground process group ID on the terminal.
pub const TIOCSPGRP: u32 = 0x00005410;
/// ioctl request: Returns the window size of the terminal.
pub const TIOCGWINSZ: u32 = 0x00005413;
/// ioctl request: Sets the window size of the terminal.
pub const TIOCSWINSZ: u32 = 0x00005414;
/// ioctl request: Returns the number of bytes available on the file descriptor.
pub const FIONREAD: u32 = 0x0000541b;

/// The implementation of the `ioctl` syscall.
pub fn ioctl(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let request = regs.ecx as u32;
	let argp = regs.edx as *const c_void;

	//crate::println!("ioctl: {} {:x} {:p}", fd, request, argp); // TODO rm

	// Getting the memory space and file
	let (mem_space, open_file_mutex) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let open_file_mutex = proc
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file();

		(mem_space, open_file_mutex)
	};

	// Getting the device file
	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

	// Executing ioctl with the current memory space
	let ret = open_file.ioctl(mem_space, request, argp)?;

	Ok(ret as _)
}
