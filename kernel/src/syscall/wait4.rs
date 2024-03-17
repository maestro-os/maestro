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

//! The `wait4` system call waits for a process to change state.

use super::waitpid;
use crate::process::{mem_space::ptr::SyscallPtr, rusage::RUsage};
use core::ffi::c_int;
use macros::syscall;
use utils::errno::Errno;

#[syscall]
pub fn wait4(
	pid: c_int,
	wstatus: SyscallPtr<c_int>,
	options: c_int,
	rusage: SyscallPtr<RUsage>,
) -> Result<i32, Errno> {
	if rusage.is_null() {
		waitpid::do_waitpid(regs, pid, wstatus, options | waitpid::WEXITED, None)
	} else {
		waitpid::do_waitpid(regs, pid, wstatus, options | waitpid::WEXITED, Some(rusage))
	}
}
