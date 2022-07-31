//! Each TTY or pseudo-TTY has to be associated with a device file in order to communicate with it.

use crate::device::DeviceHandle;
use crate::errno;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::MemSpace;
use crate::process::pid::Pid;
use crate::process::Process;
use crate::syscall::ioctl;
use crate::tty::termios::Termios;
use crate::tty::TTYHandle;
use crate::tty::WinSize;
use crate::util::io;
use crate::util::io::IO;
use crate::util::ptr::IntSharedPtr;
use core::ffi::c_void;

/// Structure representing a TTY device's handle.
pub struct TTYDeviceHandle {
	/// The device's TTY. If None, using the current process's TTY.
	tty: Option<TTYHandle>,
}

impl TTYDeviceHandle {
	/// Creates a new instance for the given TTY `tty`.
	/// If `tty` is None, the device works with the current process's TTY.
	pub fn new(tty: Option<TTYHandle>) -> Self {
		Self { tty }
	}

	/// Returns the TTY.
	fn get_tty(&self) -> Option<TTYHandle> {
		self.tty
			.clone()
			.or_else(|| Some(Process::get_current()?.lock().get().get_tty()))
	}
}

impl DeviceHandle for TTYDeviceHandle {
	fn ioctl(
		&mut self,
		mem_space: IntSharedPtr<MemSpace>,
		request: u32,
		argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO rm
		/*if matches!(request, ioctl::TCSETS | ioctl::TCSETSW | ioctl::TCSETSF) {
			let mem_space_guard = mem_space.lock();
			let termios_ptr: SyscallPtr<Termios> = (argp as usize).into();
			let termios = termios_ptr.get(&mem_space_guard)?
				.ok_or_else(|| errno!(EFAULT))?;
			crate::println!("termios: {:?}", termios);
		}*/

		let tty_mutex = self.get_tty().ok_or_else(|| errno!(ENOTTY))?;
		let tty_guard = tty_mutex.lock();
		let tty = tty_guard.get_mut();

		match request {
			ioctl::TCGETS => {
				let mem_space_guard = mem_space.lock();
				let termios_ptr: SyscallPtr<Termios> = (argp as usize).into();
				let termios_ref = termios_ptr
					.get_mut(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*termios_ref = tty.get_termios().clone();

				Ok(0)
			}

			// TODO Implement correct behaviours for each
			ioctl::TCSETS | ioctl::TCSETSW | ioctl::TCSETSF => {
				let mem_space_guard = mem_space.lock();
				let termios_ptr: SyscallPtr<Termios> = (argp as usize).into();
				let termios = termios_ptr
					.get(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				tty.set_termios(termios.clone());

				Ok(0)
			}

			ioctl::TIOCGPGRP => {
				let mem_space_guard = mem_space.lock();
				let pgid_ptr: SyscallPtr<Pid> = (argp as usize).into();
				let pgid_ref = pgid_ptr
					.get_mut(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*pgid_ref = tty.get_pgrp();

				Ok(0)
			}

			ioctl::TIOCSPGRP => {
				let mem_space_guard = mem_space.lock();
				let pgid_ptr: SyscallPtr<Pid> = (argp as usize).into();
				let pgid = pgid_ptr
					.get(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				tty.set_pgrp(*pgid);

				Ok(0)
			}

			ioctl::TIOCGWINSZ => {
				let mem_space_guard = mem_space.lock();
				let winsize: SyscallPtr<WinSize> = (argp as usize).into();
				let winsize_ref = winsize
					.get_mut(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*winsize_ref = tty.get_winsize().clone();

				Ok(0)
			}

			ioctl::TIOCSWINSZ => {
				let mem_space_guard = mem_space.lock();
				let winsize_ptr: SyscallPtr<WinSize> = (argp as usize).into();
				let winsize = winsize_ptr
					.get(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				tty.set_winsize(winsize.clone());

				Ok(0)
			}

			_ => Err(errno!(EINVAL)),
		}
	}
}

impl IO for TTYDeviceHandle {
	fn get_size(&self) -> u64 {
		if let Some(tty_mutex) = self.get_tty() {
			let tty_guard = tty_mutex.lock();
			let tty = tty_guard.get_mut();

			tty.get_available_size() as _
		} else {
			0
		}
	}

	fn read(&mut self, _offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		let tty_mutex = self.get_tty().ok_or_else(|| errno!(ENOTTY))?;
		let tty_guard = tty_mutex.lock();
		let tty = tty_guard.get_mut();

		Ok((tty.read(buff) as _, false))
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		let tty_mutex = self.get_tty().ok_or_else(|| errno!(ENOTTY))?;
		let tty_guard = tty_mutex.lock();
		let tty = tty_guard.get_mut();

		tty.write(buff);
		Ok(buff.len() as _)
	}

	fn poll(&mut self, mask: u32) -> Result<u32, Errno> {
		let tty_mutex = self.get_tty().ok_or_else(|| errno!(ENOTTY))?;
		let tty_guard = tty_mutex.lock();
		let tty = tty_guard.get_mut();

		let mut result = 0;
		if mask & io::POLLIN != 0 && tty.get_available_size() > 0 {
			result |= io::POLLIN;
		}
		if mask & io::POLLOUT != 0 {
			result |= io::POLLOUT;
		}
		// TODO Implement every events

		Ok(result)
	}
}
