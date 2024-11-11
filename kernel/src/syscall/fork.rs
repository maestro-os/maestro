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

//! The `fork` system call duplicates the whole current process into a new child
//! process. Execution resumes at the same location for both processes but the
//! return value is different to allow differentiation.

use crate::{
	arch::x86::idt::IntFrame,
	process::{ForkOptions, Process},
};
use utils::{
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};

pub fn fork(proc: Arc<Process>) -> EResult<usize> {
	let new_proc = Process::fork(proc, ForkOptions::default())?;
	Ok(new_proc.get_pid() as _)
}
