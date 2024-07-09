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

//! The `timer_settime` system call creates a per-process timer.

use crate::{
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
	time::unit::{ITimerspec32, TimerT},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// If set, the specified time is *not* relative to the timer's current counter.
const TIMER_ABSTIME: c_int = 1;

pub fn timer_settime(
	Args((timerid, flags, new_value, old_value)): Args<(
		TimerT,
		c_int,
		SyscallPtr<ITimerspec32>,
		SyscallPtr<ITimerspec32>,
	)>,
) -> EResult<usize> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mut new_value_val = new_value.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;

	let old = {
		let manager_mutex = proc.timer_manager();
		let mut manager = manager_mutex.lock();
		let timer = manager
			.get_timer_mut(timerid)
			.ok_or_else(|| errno!(EINVAL))?;

		let old = timer.get_time();
		if (flags & TIMER_ABSTIME) == 0 {
			new_value_val.it_value = new_value_val.it_value + old.it_value;
		}
		timer.set_time(new_value_val, proc.get_pid(), timerid)?;
		old
	};

	old_value.copy_to_user(old)?;

	Ok(0)
}
