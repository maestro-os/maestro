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

//! OOM killing is a procedure which is invoked when the kernel runs out of
//! memory.
//!
//! The OOM killer terminates one or more processes according to a score computed for
//! each of them.
//!
//! This is an emergency procedure which is not supposed to be used under normal conditions.

use crate::{file::vfs, memory::cache};
use utils::errno::AllocResult;

/// Attempts to reclaim memory from different places, or panics on failure.
pub fn reclaim() {
	// Attempt to shrink the page cache
	if cache::shrink() {
		return;
	}
	// Attempt to shrink the directory entries cache
	if vfs::shrink_entries() {
		return;
	}
	// TODO Attempt to:
	// - swap memory to disk
	// - if the kernel is configured for it, prompt the user to select processes to kill
	// - if the kernel is configured for it, kill the process with the highest OOM score (ignore
	//   init process)
	// - else, panic:
	panic!("Out of memory");
}

/// Executes the given function. On failure due to a lack of memory, the function runs the OOM
/// killer, then tries again.
///
/// If the OOM killer is unable to free enough memory, the kernel may panic.
pub fn wrap<T, F: FnMut() -> AllocResult<T>>(mut f: F) -> T {
	loop {
		if let Ok(r) = f() {
			return r;
		}
		reclaim();
		// TODO Check if current process has been killed
	}
}
