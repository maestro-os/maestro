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

//! The `rt_sigaction` system call sets the action for a signal.

use crate::{
	process::{
		mem_space::ptr::SyscallPtr,
		signal::{SigAction, SignalHandler},
		Process,
	},
	syscall::Signal,
};
use core::ffi::c_int;
use macros::syscall;
use utils::errno::Errno;

#[syscall]
pub fn rt_sigaction(
	signum: c_int,
	act: SyscallPtr<SigAction>,
	oldact: SyscallPtr<SigAction>,
) -> Result<i32, Errno> {
	let signal = Signal::try_from(signum as u32)?;

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap().clone();
	let mut mem_space_guard = mem_space.lock();

	// Save the old structure
	if let Some(oldact) = oldact.get_mut(&mut mem_space_guard)? {
		let action = proc.get_signal_handler(&signal).get_action();
		*oldact = action;
	}

	// Set the new structure
	if let Some(act) = act.get(&mem_space_guard)? {
		proc.set_signal_handler(&signal, SignalHandler::Handler(*act));
	}

	Ok(0)
}
