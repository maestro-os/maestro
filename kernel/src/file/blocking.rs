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
	process::{pid::Pid, scheduler, Process},
};
use core::mem;
use utils::{collections::vec::Vec, errno::EResult, lock::Mutex};

/// A queue of processes waiting on a resource.
///
/// Wait processes shall sleep, and be woken up when the resource is available.
///
/// **Note**: dropping this structure while processes are waiting on it makes them starve.
#[derive(Debug, Default)]
pub struct WaitQueue(Mutex<Vec<Pid>>); // TODO use a VecDeque

impl WaitQueue {
	/// Makes the current process wait until the given predicate `f` is true.
	pub fn wait_until<F: FnMut() -> bool>(&self, mut f: F) -> EResult<()> {
		loop {
			if f() {
				break;
			}
			// Queue
			{
				let proc_mutex = Process::current();
				let mut proc = proc_mutex.lock();
				self.0.lock().push(proc.get_pid())?;
				proc.set_state(process::State::Sleeping);
			}
			// Yield
			scheduler::end_tick();
		}
		Ok(())
	}

	/// Wakes the next process in queue.
	pub fn wake_next(&self) {
		let proc = loop {
			// TODO: inefficient, must use VecDeque
			let pid = {
				let mut pids = self.0.lock();
				if pids.is_empty() {
					// No process to wake, stop
					return;
				}
				pids.remove(0)
			};
			let Some(proc) = Process::get_by_pid(pid) else {
				// Process does not exist, try next
				continue;
			};
			break proc;
		};
		proc.lock().wake();
	}

	/// Wakes all processes.
	pub fn wake_all(&self) {
		let mut pids = self.0.lock();
		for pid in mem::take(&mut *pids) {
			let Some(proc) = Process::get_by_pid(pid) else {
				// Process does not exist, try next
				continue;
			};
			proc.lock().wake();
		}
	}
}
