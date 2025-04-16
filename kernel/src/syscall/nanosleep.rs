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

//! The `nanosleep` system call allows to make the current process sleep for a
//! given delay.

use crate::{
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
	time::{
		clock,
		clock::{current_time_ns, Clock},
		sleep_for,
		unit::{TimeUnit, Timespec32, Timestamp},
	},
};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn nanosleep(
	Args((req, rem)): Args<(SyscallPtr<Timespec32>, SyscallPtr<Timespec32>)>,
) -> EResult<usize> {
	let delay = req
		.copy_from_user()?
		.ok_or_else(|| errno!(EFAULT))?
		.to_nano();
	let mut remain = 0;
	let res = sleep_for(Clock::Monotonic, delay, &mut remain);
	match res {
		Ok(_) => Ok(0),
		Err(e) => {
			rem.copy_to_user(&Timespec32::from_nano(remain))?;
			Err(e)
		}
	}
}
