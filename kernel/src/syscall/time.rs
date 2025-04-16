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

//! The `time` syscall allows to retrieve the number of seconds elapsed since
//! the UNIX Epoch.

use crate::{
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
	time::{
		clock,
		clock::{current_time_ns, current_time_sec, Clock},
	},
};
use utils::errno::EResult;

// TODO Watch for timestamp overflow
pub fn time(Args(tloc): Args<SyscallPtr<u32>>) -> EResult<usize> {
	let time = current_time_sec(Clock::Monotonic);
	tloc.copy_to_user(&(time as _))?;
	Ok(time as _)
}
