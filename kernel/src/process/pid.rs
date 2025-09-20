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

//! PIDs handling.
//!
//! Each process must have a unique PID, thus they have to be allocated.
//! A bitfield is used to store the used PIDs.

use crate::sync::spin::Spin;
use core::{alloc::AllocError, ops::Deref};
use utils::{collections::id_allocator::IDAllocator, errno::AllocResult};

/// Type representing a Process ID. This ID is unique for every running
/// processes.
pub type Pid = u16;

/// The maximum possible PID.
const MAX_PID: Pid = 32768;
/// Special PID for the idle task.
pub const IDLE_PID: Pid = 0;
/// PID of the init process.
pub const INIT_PID: Pid = 1;

/// The PID allocator.
static ALLOCATOR: Spin<Option<IDAllocator>> = Spin::new(None);

/// Perform an operation with the allocator.
fn allocator_do<F: Fn(&mut IDAllocator) -> AllocResult<T>, T>(f: F) -> AllocResult<T> {
	let mut allocator = ALLOCATOR.lock();
	let allocator = match &mut *allocator {
		Some(a) => a,
		None => allocator.insert(IDAllocator::new(MAX_PID as _)?),
	};
	f(allocator)
}

/// Wrapper for a PID, freeing it on drop.
#[derive(Debug)]
pub struct PidHandle(Pid);

impl PidHandle {
	/// Allocates the given `pid`.
	///
	/// If already allocated, the function returns an error.
	pub(super) fn mark_used(pid: Pid) -> AllocResult<Self> {
		let Some(id) = pid.checked_sub(1) else {
			// Pid `0` is not allocated, just return a handle
			return Ok(Self(pid));
		};
		allocator_do(|a| {
			if !a.is_used(id as _) {
				a.set_used(id as _);
				Ok(Self(pid))
			} else {
				Err(AllocError)
			}
		})
	}

	/// Returns an unused PID and marks it as used.
	pub fn unique() -> AllocResult<PidHandle> {
		allocator_do(|allocator| allocator.alloc(None)).map(|i| PidHandle((i + 1) as _))
	}
}

impl Deref for PidHandle {
	type Target = Pid;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Drop for PidHandle {
	fn drop(&mut self) {
		// Cannot free PID `0`
		let Some(i) = self.0.checked_sub(1) else {
			return;
		};
		// Cannot fail
		let _ = allocator_do(|a| {
			a.free(i as _);
			Ok(())
		});
	}
}
