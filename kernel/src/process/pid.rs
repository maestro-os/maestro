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

use utils::{collections::id_allocator::IDAllocator, errno::AllocResult, lock::Mutex};

/// Type representing a Process ID. This ID is unique for every running
/// processes.
pub type Pid = u16;

/// The maximum possible PID.
const MAX_PID: Pid = 32768;
/// The PID of the init process.
pub const INIT_PID: Pid = 1;

/// The PID allocator.
static ALLOCATOR: Mutex<Option<IDAllocator>> = Mutex::new(None);

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
	/// Returns the init PID.
	///
	/// This function **must not** be used outside the creation of the first process.
	pub(super) fn init() -> AllocResult<Self> {
		allocator_do(|a| {
			a.set_used((INIT_PID - 1) as _);
			Ok(())
		})?;
		Ok(Self(INIT_PID))
	}

	/// Returns an unused PID and marks it as used.
	pub fn unique() -> AllocResult<PidHandle> {
		allocator_do(|allocator| allocator.alloc(None)).map(|i| PidHandle((i + 1) as _))
	}

	/// Returns the actual PID.
	pub fn get(&self) -> Pid {
		self.0
	}
}

impl Drop for PidHandle {
	fn drop(&mut self) {
		// Cannot fail
		let _ = allocator_do(|a| {
			a.free((self.0 - 1) as _);
			Ok(())
		});
	}
}
