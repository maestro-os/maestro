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

//! The `vfork` system call works the same as the `fork` system call, except the
//! parent process is blocked until the child process exits or executes a
//! program. During that time, the child process also shares the same memory
//! space as the parent.

use crate::{
	arch::x86::idt::IntFrame,
	memory::user::UserPtr,
	process::{ForkOptions, Process, scheduler, scheduler::Scheduler},
	syscall::{
		Args,
		clone::{CLONE_VFORK, CLONE_VM, clone},
	},
};
use core::ptr::null_mut;
use utils::{
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn vfork(proc: Arc<Process>, frame: &mut IntFrame) -> EResult<usize> {
	clone(
		Args((
			CLONE_VFORK | CLONE_VM,
			null_mut(),
			UserPtr(None),
			UserPtr(None),
			0,
		)),
		proc,
		frame,
	)
}
