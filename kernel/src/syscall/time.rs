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
	process::{
		mem_space::copy::UserPtr,
		signal::{SigEvent, Signal, SIGEV_SIGNAL},
		Process,
	},
	syscall::Args,
	time::{
		clock,
		clock::{current_time_ns, current_time_sec, Clock},
		sleep_for,
		unit::{ClockIdT, ITimerspec32, TimeUnit, TimerT, Timespec, Timespec32},
	},
};
use core::ffi::c_int;
use utils::{errno, errno::EResult, ptr::arc::Arc};

/// If set, the specified time is *not* relative to the timer's current counter.
const TIMER_ABSTIME: c_int = 1;

pub fn time32(Args(tloc): Args<UserPtr<u32>>) -> EResult<usize> {
	let time = current_time_sec(Clock::Monotonic);
	let time: u32 = time.try_into().map_err(|_| errno!(EOVERFLOW))?;
	tloc.copy_to_user(&time)?;
	Ok(time as _)
}

pub fn time64(Args(tloc): Args<UserPtr<u64>>) -> EResult<usize> {
	let time = current_time_sec(Clock::Monotonic);
	tloc.copy_to_user(&time)?;
	Ok(time as _)
}

pub fn clock_gettime(Args((clockid, tp)): Args<(ClockIdT, UserPtr<Timespec>)>) -> EResult<usize> {
	let clk = Clock::from_id(clockid).ok_or_else(|| errno!(EINVAL))?;
	let ts = current_time_ns(clk);
	tp.copy_to_user(&Timespec::from_nano(ts))?;
	Ok(0)
}

pub fn clock_gettime64(
	Args((clockid, tp)): Args<(ClockIdT, UserPtr<Timespec>)>,
) -> EResult<usize> {
	let clock = Clock::from_id(clockid).ok_or_else(|| errno!(EINVAL))?;
	let ts = current_time_ns(clock);
	tp.copy_to_user(&Timespec::from_nano(ts))?;
	Ok(0)
}

pub fn nanosleep32(
	Args((req, rem)): Args<(UserPtr<Timespec32>, UserPtr<Timespec32>)>,
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

pub fn nanosleep64(
	Args((req, rem)): Args<(UserPtr<Timespec>, UserPtr<Timespec>)>,
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
			rem.copy_to_user(&Timespec::from_nano(remain))?;
			Err(e)
		}
	}
}

pub fn timer_create(
	Args((clockid, sevp, timerid)): Args<(ClockIdT, UserPtr<SigEvent>, UserPtr<TimerT>)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	let clock = Clock::from_id(clockid).ok_or_else(|| errno!(EINVAL))?;
	let timerid_val = timerid.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let sevp_val = sevp.copy_from_user()?.unwrap_or_else(|| SigEvent {
		sigev_notify: SIGEV_SIGNAL,
		sigev_signo: Signal::SIGALRM as _,
		sigev_value: timerid_val,
		sigev_notify_function: None,
		sigev_notify_attributes: None,
		sigev_notify_thread_id: proc.tid,
	});
	let id = proc.timer_manager.lock().create_timer(clock, sevp_val)?;
	timerid.copy_to_user(&(id as _))?;
	Ok(0)
}

pub fn timer_delete(Args(timerid): Args<TimerT>, proc: Arc<Process>) -> EResult<usize> {
	proc.timer_manager.lock().delete_timer(timerid)?;
	Ok(0)
}

pub fn timer_settime(
	Args((timerid, flags, new_value, old_value)): Args<(
		TimerT,
		c_int,
		UserPtr<ITimerspec32>,
		UserPtr<ITimerspec32>,
	)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Get timer
	let mut manager = proc.timer_manager.lock();
	let timer = manager
		.get_timer_mut(timerid)
		.ok_or_else(|| errno!(EINVAL))?;
	// Write old value
	let old = timer.get_time();
	old_value.copy_to_user(&old)?;
	// Set new value
	let mut new_value_val = new_value.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	if (flags & TIMER_ABSTIME) == 0 {
		new_value_val.it_value = new_value_val.it_value + old.it_value;
	}
	timer.set_time(
		new_value_val.it_interval.to_nano(),
		new_value_val.it_value.to_nano(),
	)?;
	Ok(0)
}
