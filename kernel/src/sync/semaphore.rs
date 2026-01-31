/*
 * Copyright 2026 Luc Lenôtre
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

//! Sleeping semaphore synchronization primitive.
//!
//! A semaphore has a number of permits. When trying to acquire a permit while none is available,
//! the semaphore makes the process sleep until a permit becomes available.

use crate::{
	process,
	process::{Process, State, scheduler::schedule},
	sync::spin::IntSpin,
};
use utils::{errno, errno::EResult, list, list_type};

fn acquire<const INT: bool>(queue: &IntSpin<Queue>, permits: usize) -> EResult<()> {
	{
		let mut q = queue.lock();
		q.acquired += 1;
		// If enough permits are available, return
		if q.acquired < permits {
			return Ok(());
		}
		// Not enough permits: we must sleep. The process is dequeued when a permit becomes
		// available for this process
		q.wait_queue.insert_back(Process::current());
		// Put to sleep before releasing the spinlock to make sure another process releasing
		// a permit does not try to wake us up before we sleep
		if INT {
			process::set_state(State::IntSleeping);
		} else {
			process::set_state(State::Sleeping);
		}
	}
	schedule();
	let proc = Process::current();
	let mut q = queue.lock();
	// Make sure the process is dequeued
	unsafe {
		q.wait_queue.remove(&proc);
	}
	// If woken up by a signal
	if INT && proc.has_pending_signal() {
		// Release the permit
		q.acquired -= 1;
		return Err(errno!(EINTR));
	}
	Ok(())
}

/// Releases the permit when dropped
pub struct SemaphoreGuard<'m, const INT: bool> {
	sem: &'m Semaphore<INT>,
}

impl<const INT: bool> !Send for SemaphoreGuard<'_, INT> {}

unsafe impl<const INT: bool> Sync for SemaphoreGuard<'_, INT> {}

impl<const INT: bool> Drop for SemaphoreGuard<'_, INT> {
	fn drop(&mut self) {
		unsafe {
			self.sem.release();
		}
	}
}

struct Queue {
	acquired: usize,
	wait_queue: list_type!(Process, wait_queue),
}

/// Sleeping semaphore.
///
/// `INT` tells whether sleeping can be interrupted by a signal.
pub struct Semaphore<const INT: bool = true> {
	permits: usize,
	queue: IntSpin<Queue>,
}

impl<const INT: bool> Semaphore<INT> {
	/// Creates a new instance with the given amount of permits.
	pub const fn new(permits: usize) -> Self {
		Self {
			permits,
			queue: IntSpin::new(Queue {
				acquired: 0,
				wait_queue: list!(Process, wait_queue),
			}),
		}
	}
}

impl<const INT: bool> Semaphore<INT> {
	/// Releases a permit, waking up the next process waiting for one, if any.
	///
	/// # Safety
	///
	/// This function should not be used directly since it is called when the guard is dropped.
	///
	/// If no permit is taken, the behaviour is undefined.
	pub unsafe fn release(&self) {
		let next = {
			let mut q = self.queue.lock();
			q.acquired -= 1;
			// If at least one other task is waiting, wake it up
			q.wait_queue.remove_front()
		};
		if let Some(next) = next {
			let mut mask = State::Sleeping as u8;
			if INT {
				mask |= State::IntSleeping as u8;
			}
			Process::wake_from(&next, mask);
		}
	}
}

impl Semaphore<false> {
	/// Acquires a permit.
	///
	/// If no permit is available, the thread sleeps until one becomes available.
	///
	/// The function returns a [`SemaphoreGuard`] associated with `self`. When dropped, the permit
	/// is released.
	pub fn acquire(&self) -> SemaphoreGuard<false> {
		let _ = acquire::<false>(&self.queue, self.permits);
		SemaphoreGuard {
			sem: self,
		}
	}
}

impl Semaphore<true> {
	/// Acquires a permit.
	///
	/// If no permit is available, the thread sleeps until one becomes available.
	///
	/// The function returns a [`SemaphoreGuard`] associated with `self`. When dropped, the permit
	/// is released.
	///
	/// If the current process is interrupted by a signal while waiting, the function returns with
	/// the errno [`errno::EINTR`].
	pub fn acquire(&self) -> EResult<SemaphoreGuard<true>> {
		acquire::<true>(&self.queue, self.permits)?;
		Ok(SemaphoreGuard {
			sem: self,
		})
	}
}

unsafe impl<const INT: bool> Sync for Semaphore<INT> {}
