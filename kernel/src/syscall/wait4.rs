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

use super::{waitpid, Args};
use crate::process::{mem_space::copy::SyscallPtr, regs::Regs, rusage::RUsage};
use core::ffi::c_int;
use utils::errno::EResult;

pub fn wait4(
	Args((pid, wstatus, options, rusage)): Args<(
		c_int,
		SyscallPtr<c_int>,
		c_int,
		SyscallPtr<RUsage>,
	)>,
	regs: &Regs,
) -> EResult<usize> {
	waitpid::do_waitpid(regs, pid, wstatus, options | waitpid::WEXITED, rusage)
}
