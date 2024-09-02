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

//! Utility functions for system calls.

pub mod at;

use crate::process::{regs::Regs, scheduler, Process, State};

/// Checks whether the current syscall must be interrupted to execute a signal.
///
/// If a signal has to be handled, the function abort the execution of the system call. Then the
/// signal is executed.
///
/// If the signal handler has the [`SA_RESTART`] flag set, the system call is restarted after the
/// signal handler returns. If not set, the system call returns [`EINTR`].
///
/// The function locks the mutex of the current process. Thus, the caller must
/// ensure the mutex isn't already locked to prevent a deadlock.
///
/// `regs` is the registers state passed to the current syscall.
pub fn handle_signal(regs: &Regs) {
	let regs = {
		let proc_mutex = Process::current();
		let mut proc = proc_mutex.lock();
		// If no signal is pending, return
		let Some(sig) = proc.next_signal(false) else {
			return;
		};
		// Prepare signal for execution
		let handlers = proc.signal_handlers.clone();
		let handlers = handlers.lock();
		let handler = &handlers[sig.get_id() as usize];
		// Update registers with the ones passed to the system call so that `sigreturn` returns to
		// the correct location
		proc.regs = regs.clone();
		handler.exec(sig, &mut proc);
		// Alter the execution flow of the current context according to the new state of the
		// process
		match proc.get_state() {
			// The process must execute a signal handler. Jump to it
			State::Running => Some(proc.regs.clone()),
			// Stop execution. Waiting until wakeup (or terminate if Zombie)
			State::Sleeping | State::Stopped | State::Zombie => None,
		}
	};
	match regs {
		Some(regs) => unsafe {
			regs.switch(true);
		},
		None => scheduler::end_tick(),
	}
}
