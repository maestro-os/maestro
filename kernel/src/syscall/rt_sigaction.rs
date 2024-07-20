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
		mem_space::copy::SyscallPtr,
		signal::{SigAction, SignalHandler},
		Process,
	},
	syscall::{Args, Signal},
};
use core::ffi::c_int;
use utils::{
	errno::EResult,
	lock::{IntMutex, IntMutexGuard},
	ptr::arc::Arc,
};

pub fn rt_sigaction(
	Args((signum, act, oldact)): Args<(c_int, SyscallPtr<SigAction>, SyscallPtr<SigAction>)>,
	proc: Arc<IntMutex<Process>>,
) -> EResult<usize> {
	// Validation
	let signal = Signal::try_from(signum)?;
	let signal_handlers_mutex = proc.lock().signal_handlers.clone();
	let mut signal_handlers = signal_handlers_mutex.lock();
	// Save the old structure
	let old = signal_handlers[signal.get_id() as usize].get_action();
	oldact.copy_to_user(old)?;
	// Set the new structure
	if let Some(new) = act.copy_from_user()? {
		signal_handlers[signal.get_id() as usize] = SignalHandler::Handler(new);
	}
	Ok(0)
}
