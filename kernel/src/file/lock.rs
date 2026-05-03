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

//! Advisory file locking

use crate::sync::wait_queue::WaitQueue;
use core::{
	hint::unlikely,
	sync::atomic::{
		AtomicUsize,
		Ordering::{Acquire, Relaxed, Release},
	},
};
use utils::{errno, errno::EResult};

const EXCLUSIVE_LOCKED: usize = !0;

/// The lock mode currently held by an open file description.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum FlockMode {
	/// No lock held
	#[default]
	None,
	/// Shared  lock
	Shared,
	/// Exclusive lock
	Exclusive,
}

/// BSD-style `flock` handle, attached to an inode.
///
/// The actual conflict state lives here; per-open-file-description bookkeeping (which
/// mode this OFD currently holds) is tracked separately on the [`crate::file::File`].
#[derive(Debug, Default)]
pub struct Flock {
	/// The number of shared leases taken. If an exclusive lease is taken, the value is
	/// [`EXCLUSIVE_LOCKED`].
	leases: AtomicUsize,
	/// Processes waiting on the lock.
	wait_queue: WaitQueue,
}

impl Flock {
	fn try_acquire(&self, exclusive: bool) -> EResult<bool> {
		let mut overflow = false;
		let res = self.leases.fetch_update(Acquire, Relaxed, |val| {
			if val == EXCLUSIVE_LOCKED {
				return None;
			}
			if exclusive {
				(val == 0).then_some(EXCLUSIVE_LOCKED)
			} else {
				let next = val + 1;
				if next == EXCLUSIVE_LOCKED {
					overflow = true;
					return None;
				}
				Some(next)
			}
		});
		if unlikely(overflow) {
			return Err(errno!(ENOLCK));
		}
		Ok(res.is_ok())
	}

	/// Acquires a lease, blocking if necessary.
	///
	/// If `non_blocking` is `true` and the lease cannot be taken immediately, returns
	/// [`errno::EWOULDBLOCK`].
	pub fn acquire(&self, exclusive: bool, non_blocking: bool) -> EResult<()> {
		if self.try_acquire(exclusive)? {
			return Ok(());
		}
		if non_blocking {
			return Err(errno!(EWOULDBLOCK));
		}
		self.wait_queue
			.wait_until(|| match self.try_acquire(exclusive) {
				Ok(true) => Some(Ok(())),
				Ok(false) => None,
				Err(e) => Some(Err(e)),
			})?
	}

	/// Releases a lease previously taken in the given `mode`.
	///
	/// Does nothing if `mode` is [`FlockMode::None`].
	pub fn release(&self, mode: FlockMode) {
		match mode {
			FlockMode::None => return,
			FlockMode::Shared => {
				self.leases.fetch_sub(1, Release);
			}
			FlockMode::Exclusive => {
				self.leases.swap(0, Release);
			}
		}
		self.wait_queue.wake_all();
	}
}
