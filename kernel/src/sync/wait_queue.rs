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

//! Queue of processes waiting on a resource.

use crate::{
	process,
	process::{Process, State, scheduler::schedule},
	sync::spin::IntSpin,
};
use core::{fmt, fmt::Formatter};
use utils::{errno, errno::EResult, list, list_type};

/// Queue of processes waiting on a resource.
///
/// While waiting, the process is turned to the [`IntSleeping`] or [`Sleeping`] state.
pub struct WaitQueue(IntSpin<list_type!(Process, wait_queue)>);

impl Default for WaitQueue {
	fn default() -> Self {
		Self::new()
	}
}

impl WaitQueue {
	/// Creates a new empty queue.
	pub const fn new() -> Self {
		Self(IntSpin::new(list!(Process, wait_queue)))
	}

	/// Makes the current process wait (sleep) until woken up.
	///
	/// If the process has been interrupted while waiting, the function returns [`EINTR`].
	pub fn wait(&self) -> EResult<()> {
		// Enqueue and put the process to sleep
		self.0.lock().insert_back(Process::current());
		process::set_state(State::IntSleeping);
		// Switch context
		schedule();
		{
			let proc = Process::current();
			// Make sure the process is dequeued
			unsafe {
				self.0.lock().remove(&proc);
			}
			// If woken up by a signal
			if proc.has_pending_signal() {
				return Err(errno!(EINTR));
			}
		}
		Ok(())
	}

	/// Makes the current process wait until the given closure returns `Some`.
	///
	/// If waiting is interrupted by a signal handler, the function returns [`errno::EINTR`].
	pub fn wait_until<F: FnMut() -> Option<T>, T>(&self, mut f: F) -> EResult<T> {
		loop {
			if let Some(val) = f() {
				break Ok(val);
			}
			self.wait()?;
		}
	}

	/// Wakes the next process in queue, if any.
	pub fn wake_next(&self) {
		if let Some(proc) = self.0.lock().remove_front() {
			Process::wake_from(&proc, State::IntSleeping as u8);
		}
	}

	/// Wakes all processes in queue, if any.
	pub fn wake_all(&self) {
		let mut queue = self.0.lock();
		for node in queue.iter() {
			let proc = node.remove();
			Process::wake_from(&proc, State::IntSleeping as u8);
		}
	}
}

unsafe impl Sync for WaitQueue {}

impl fmt::Debug for WaitQueue {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str("WaitQueue")
	}
}
