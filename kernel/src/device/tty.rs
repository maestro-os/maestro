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
	device::DeviceIO,
	process::{
		mem_space::copy::SyscallPtr,
		pid::Pid,
		signal::{Signal, SignalHandler},
		Process,
	},
	syscall::{ioctl, FromSyscallArg},
	tty::{termios, termios::Termios, TTYDisplay, WinSize, TTY},
};
use core::{ffi::c_void, num::NonZeroU64};
use utils::{errno, errno::EResult};

/// A TTY device's handle.
pub struct TTYDeviceHandle;

impl TTYDeviceHandle {
	/// Checks whether the process is allowed to read from the TTY.
	///
	/// If not, it is killed with a `SIGTTIN` signal.
	///
	/// Arguments:
	/// - `process` is the process.
	/// - `tty` is the TTY.
	///
	/// This function must be called before performing the read operation.
	fn check_sigttin(&self, proc: &mut Process, tty: &TTYDisplay) -> EResult<()> {
		if proc.pgid == tty.get_pgrp() {
			return Ok(());
		}
		// Hold the signal handlers table to avoid a race condition
		let signal_handlers = proc.signal_handlers.clone();
		let signal_handlers = signal_handlers.lock();
		let handler = &signal_handlers[Signal::SIGTTIN.get_id() as usize];
		if proc.is_signal_blocked(Signal::SIGTTIN)
			|| matches!(handler, SignalHandler::Ignore)
			|| proc.is_in_orphan_process_group()
		{
			return Err(errno!(EIO));
		}
		proc.kill_group(Signal::SIGTTIN);
		Ok(())
	}

	/// Checks whether the process is allowed to write to the TTY.
	///
	/// If not, it is killed with a `SIGTTOU` signal.
	///
	/// Arguments:
	/// - `process` is the process.
	/// - `tty` is the TTY.
	///
	/// This function must be called before performing the write operation.
	fn check_sigttou(&self, proc: &mut Process, tty: &TTYDisplay) -> EResult<()> {
		if tty.get_termios().c_lflag & termios::consts::TOSTOP == 0 {
			return Ok(());
		}
		// Hold the signal handlers table to avoid a race condition
		let signal_handlers = proc.signal_handlers.clone();
		let signal_handlers = signal_handlers.lock();
		let handler = &signal_handlers[Signal::SIGTTOU.get_id() as usize];
		if proc.is_signal_blocked(Signal::SIGTTOU) || matches!(handler, SignalHandler::Ignore) {
			return Ok(());
		}
		if proc.is_in_orphan_process_group() {
			return Err(errno!(EIO));
		}
		proc.kill_group(Signal::SIGTTOU);
		Ok(())
	}
}

impl DeviceIO for TTYDeviceHandle {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		0
	}

	fn read(&self, _off: u64, buff: &mut [u8]) -> EResult<usize> {
		let proc_mutex = Process::current();
		let mut proc = proc_mutex.lock();
		self.check_sigttin(&mut proc, &TTY.display.lock())?;
		let len = TTY.read(buff)?;
		Ok(len)
	}

	fn write(&self, _off: u64, buff: &[u8]) -> EResult<usize> {
		let proc_mutex = Process::current();
		let mut proc = proc_mutex.lock();
		let mut tty = TTY.display.lock();
		self.check_sigttou(&mut proc, &tty)?;
		tty.write(buff);
		Ok(buff.len())
	}

	fn ioctl(&self, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		let proc_mutex = Process::current();
		let mut proc = proc_mutex.lock();
		let mut tty = TTY.display.lock();
		match request.get_old_format() {
			ioctl::TCGETS => {
				let termios_ptr = SyscallPtr::<Termios>::from_syscall_arg(argp as usize);
				termios_ptr.copy_to_user(tty.get_termios().clone())?;
				Ok(0)
			}
			// TODO Implement correct behaviours for each
			ioctl::TCSETS | ioctl::TCSETSW | ioctl::TCSETSF => {
				self.check_sigttou(&mut proc, &tty)?;
				let termios_ptr = SyscallPtr::<Termios>::from_syscall_arg(argp as usize);
				let termios = termios_ptr
					.copy_from_user()?
					.ok_or_else(|| errno!(EFAULT))?;
				tty.set_termios(termios.clone());
				Ok(0)
			}
			ioctl::TIOCGPGRP => {
				let pgid_ptr = SyscallPtr::<Pid>::from_syscall_arg(argp as usize);
				pgid_ptr.copy_to_user(tty.get_pgrp())?;
				Ok(0)
			}
			ioctl::TIOCSPGRP => {
				self.check_sigttou(&mut proc, &tty)?;
				let pgid_ptr = SyscallPtr::<Pid>::from_syscall_arg(argp as usize);
				let pgid = pgid_ptr.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
				tty.set_pgrp(pgid);
				Ok(0)
			}
			ioctl::TIOCGWINSZ => {
				let winsize = SyscallPtr::<WinSize>::from_syscall_arg(argp as usize);
				winsize.copy_to_user(tty.get_winsize().clone())?;
				Ok(0)
			}
			ioctl::TIOCSWINSZ => {
				let winsize_ptr = SyscallPtr::<WinSize>::from_syscall_arg(argp as usize);
				let winsize = winsize_ptr
					.copy_from_user()?
					.ok_or_else(|| errno!(EFAULT))?;
				// Drop to avoid deadlock since `set_winsize` sends the SIGWINCH signal
				drop(proc);
				tty.set_winsize(winsize.clone());
				Ok(0)
			}
			_ => Err(errno!(EINVAL)),
		}
	}
}
