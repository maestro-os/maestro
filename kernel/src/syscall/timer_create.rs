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

//! The `timer_create` system call creates a per-process timer.

use crate::{
	process::{
		mem_space::ptr::SyscallPtr,
		signal::{SigEvent, SigVal, Signal, SIGEV_SIGNAL},
		Process,
	},
	time::unit::{ClockIdT, TimerT},
};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn timer_create(
	clockid: ClockIdT,
	sevp: SyscallPtr<SigEvent>,
	timerid: SyscallPtr<TimerT>,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	let timerid_val = *timerid
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;

	let sevp_val = sevp
		.get(&mem_space_guard)?
		.cloned()
		.unwrap_or_else(|| SigEvent {
			sigev_notify: SIGEV_SIGNAL,
			sigev_signo: Signal::SIGALRM.get_id() as _,
			sigev_value: SigVal {
				sigval_ptr: timerid_val,
			},
			sigev_notify_function: None,
			sigev_notify_attributes: None,
			sigev_notify_thread_id: proc.tid,
		});

	let id = proc
		.timer_manager()
		.lock()
		.create_timer(clockid, sevp_val)?;

	// Return timer ID
	let timerid_val = timerid
		.get_mut(&mut mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	*timerid_val = id as _;

	Ok(0)
}
