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

//! When a resource is blocking, a process trying to use it must be put in `Sleeping` state until
//! the resource is available.

use crate::{
	process,
	process::{pid::Pid, Process},
};
use utils::{collections::hashmap::HashMap, errno::EResult, io};

/// Handler allowing to make a process sleep when waiting on a resource, then resume its execution
/// when the resource is available.
#[derive(Debug, Default)]
pub struct BlockHandler {
	/// The list of processes waiting on the resource, along with the mask of events to wait for.
	waiting_procs: HashMap<Pid, u32>,
}

impl BlockHandler {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			waiting_procs: HashMap::new(),
		}
	}

	/// Adds the given process to the list of processes waiting on the resource.
	///
	/// The function sets the state of the process to `Sleeping`.
	/// When the event occurs, the process will be woken up.
	///
	/// `mask` is the mask of poll event to wait for.
	pub fn add_waiting_process(&mut self, proc: &mut Process, mask: u32) -> EResult<()> {
		self.waiting_procs.insert(proc.pid, mask)?;
		proc.set_state(process::State::Sleeping);

		Ok(())
	}

	/// Wakes processes for the events in the given mask.
	pub fn wake_processes(&mut self, mask: u32) {
		self.waiting_procs.retain(|pid, m| {
			let Some(proc_mutex) = Process::get_by_pid(*pid) else {
				return false;
			};

			let wake = mask & *m != 0;
			if !wake {
				return true;
			}

			let mut proc = proc_mutex.lock();
			proc.wake();

			false
		});
	}
}

impl Drop for BlockHandler {
	fn drop(&mut self) {
		self.wake_processes(io::POLLERR);
	}
}
