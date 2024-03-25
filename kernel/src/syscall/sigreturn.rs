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

//! The `sigreturn` system call is used whenever the process finished executing
//! a signal handler.
//!
//! The system call restores the previous state of the process
//! to allow normal execution.

use crate::process::Process;
use macros::syscall;
use utils::{errno::Errno, interrupt::cli};

#[syscall]
pub fn sigreturn() -> EResult<i32> {
	cli();
	let regs = {
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();
		// Restores the state of the process before the signal handler
		proc.signal_restore();
		proc.regs.clone()
	};
	// Resume execution
	unsafe {
		regs.switch(true);
	}
}
