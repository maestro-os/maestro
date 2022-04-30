//! Each TTY or pseudo-TTY has to be associated with a device file in order to communicate with it.

use core::ffi::c_void;
use crate::device::DeviceHandle;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::mem_space::MemSpace;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::syscall::ioctl;
use crate::tty::TTY;
use crate::tty::WinSize;
use crate::tty;
use crate::util::IO;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;

/// Structure representing a TTY device's handle.
pub struct TTYDeviceHandle {
	/// The device's TTY. If None, using the current process's TTY.
	tty: Option<SharedPtr<TTY>>,
}

impl TTYDeviceHandle {
	/// Creates a new instance for the given TTY `tty`.
	/// If `tty` is None, the device works with the current process's TTY.
	pub fn new(tty: Option<SharedPtr<TTY>>) -> Self {
		Self {
			tty,
		}
	}

	/// Returns the TTY.
	fn get_tty(&self) -> SharedPtr<TTY> {
		self.tty.clone()
			.or_else(|| {
				Process::get_current()?.lock().get().get_tty()
			})
			.unwrap_or_else(|| tty::current())
	}
}

impl DeviceHandle for TTYDeviceHandle {
	fn ioctl(&mut self, mem_space: IntSharedPtr<MemSpace>, request: u32, argp: *const c_void)
		-> Result<u32, Errno> {
		let mut tty_guard = self.get_tty().lock();
		let tty = tty_guard.get_mut();

		match request {
			ioctl::TIOCGPGRP => {
				// TODO
				todo!();
			},

			ioctl::TIOCSPGRP => {
				// TODO
				todo!();
			},

			ioctl::TIOCGWINSZ => {
				let mem_space_guard = mem_space.lock();
				let winsize: SyscallPtr<WinSize> = (argp as usize).into();
				let winsize_ref = winsize.get_mut(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*winsize_ref = tty.get_winsize();

				Ok(0)
			},

			_ => Err(errno!(EINVAL)),
		}
	}
}

impl IO for TTYDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<u64, Errno> {
		// TODO Read from TTY input
		todo!();
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		self.get_tty().lock().get_mut().write(buff);
		Ok(buff.len() as _)
	}
}
