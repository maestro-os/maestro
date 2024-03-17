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

//! The `getegid` syscall returns the effective GID of the process's owner.

use crate::process::{regs::Regs, Process};
use utils::errno::EResult;

pub fn getegid(_: &Regs) -> EResult<i32> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();
	Ok(proc.access_profile.get_egid() as _)
}
