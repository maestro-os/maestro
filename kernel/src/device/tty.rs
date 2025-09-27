/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Each TTY or pseudo-TTY has to be associated with a device file in order to
//! communicate with it.

use crate::{
	file::{File, fs::FileOps},
	memory::user::{UserPtr, UserSlice},
	process::{
		Process,
		pid::Pid,
		signal::{Signal, SignalHandler},
	},
	syscall::{
		FromSyscallArg, ioctl,
		select::{POLLIN, POLLOUT},
	},
	tty::{TTY, WinSize, termios, termios::Termios},
};
use core::ffi::c_void;
use utils::{errno, errno::EResult};

/// A TTY device's handle.
#[derive(Debug)]
pub struct TTYDeviceHandle;

impl TTYDeviceHandle {
	/// Checks whether the current process is allowed to read from the TTY.
	///
	/// If not, it is killed with a `SIGTTIN` signal.
	///
	/// This function must be called before performing the read operation.
	fn check_sigttin(&self) -> EResult<()> {
		let proc = Process::current();
		if proc.get_pgid() == TTY.get_pgrp() {
			return Ok(());
		}
		if proc.is_in_orphan_process_group() {
			return Err(errno!(EIO));
		}
		if proc.signal.lock().is_signal_blocked(Signal::SIGTTIN) {
			return Err(errno!(EIO));
		}
		if matches!(
			proc.sig_handlers.lock()[Signal::SIGTTIN as usize],
			SignalHandler::Ignore
		) {
			return Err(errno!(EIO));
		}
		proc.kill_group(Signal::SIGTTIN);
		Ok(())
	}

	/// Checks whether the current process is allowed to write to the TTY.
	///
	/// If not, it is killed with a `SIGTTOU` signal.
	///
	/// This function must be called before performing the write operation.
	fn check_sigttou(&self) -> EResult<()> {
		let proc = Process::current();
		if TTY.get_termios().c_lflag & termios::consts::TOSTOP == 0 {
			return Ok(());
		}
		if proc.signal.lock().is_signal_blocked(Signal::SIGTTOU) {
			return Err(errno!(EIO));
		}
		if matches!(
			proc.sig_handlers.lock()[Signal::SIGTTOU as usize],
			SignalHandler::Ignore
		) {
			return Err(errno!(EIO));
		}
		if proc.is_in_orphan_process_group() {
			return Err(errno!(EIO));
		}
		proc.kill_group(Signal::SIGTTOU);
		Ok(())
	}
}

impl FileOps for TTYDeviceHandle {
	fn poll(&self, _file: &File, mask: u32) -> EResult<u32> {
		let input = TTY.has_input_available();
		let res = (if input { POLLIN } else { 0 } | POLLOUT) & mask;
		Ok(res)
	}

	fn ioctl(&self, _file: &File, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		match request.get_old_format() {
			ioctl::TCGETS => {
				let termios_ptr = UserPtr::<Termios>::from_ptr(argp as usize);
				termios_ptr.copy_to_user(&TTY.get_termios())?;
				Ok(0)
			}
			// TODO Implement correct behaviours for each
			ioctl::TCSETS | ioctl::TCSETSW | ioctl::TCSETSF => {
				self.check_sigttou()?;
				let termios_ptr = UserPtr::<Termios>::from_ptr(argp as usize);
				let termios = termios_ptr
					.copy_from_user()?
					.ok_or_else(|| errno!(EFAULT))?;
				TTY.set_termios(termios.clone());
				Ok(0)
			}
			ioctl::TIOCGPGRP => {
				let pgid_ptr = UserPtr::<Pid>::from_ptr(argp as usize);
				pgid_ptr.copy_to_user(&TTY.get_pgrp())?;
				Ok(0)
			}
			ioctl::TIOCSPGRP => {
				self.check_sigttou()?;
				let pgid_ptr = UserPtr::<Pid>::from_ptr(argp as usize);
				let pgid = pgid_ptr.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
				TTY.set_pgrp(pgid);
				Ok(0)
			}
			ioctl::TIOCGWINSZ => {
				let winsize = UserPtr::<WinSize>::from_ptr(argp as usize);
				winsize.copy_to_user(&TTY.get_winsize())?;
				Ok(0)
			}
			ioctl::TIOCSWINSZ => {
				let winsize_ptr = UserPtr::<WinSize>::from_ptr(argp as usize);
				let winsize = winsize_ptr
					.copy_from_user()?
					.ok_or_else(|| errno!(EFAULT))?;
				TTY.set_winsize(winsize.clone());
				Ok(0)
			}
			_ => Err(errno!(EINVAL)),
		}
	}

	fn read(&self, _file: &File, _off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		self.check_sigttin()?;
		let len = TTY.read(buf)?;
		Ok(len)
	}

	fn write(&self, _file: &File, _off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		self.check_sigttou()?;
		// Write
		let mut i = 0;
		let mut b: [u8; 128] = [0; 128];
		while i < buf.len() {
			let l = buf.copy_from_user(i, &mut b)?;
			TTY.write(&b[..l]);
			i += l;
		}
		Ok(buf.len())
	}
}
