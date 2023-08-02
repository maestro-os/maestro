//! The `timer_settime` system call creates a per-process timer.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::time::unit::ITimerspec32;
use crate::time::unit::TimerT;
use core::ffi::c_int;
use macros::syscall;

/// If set, the specified time is *not* relative to the timer's current counter.
const TIMER_ABSTIME: c_int = 1;

#[syscall]
pub fn timer_settime(
	timerid: TimerT,
	flags: c_int,
	new_value: SyscallPtr<ITimerspec32>,
	old_value: SyscallPtr<ITimerspec32>,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	let mut new_value_val = new_value
		.get(&mem_space_guard)?
		.cloned()
		.ok_or_else(|| errno!(EFAULT))?;

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
		timer.set_time(new_value_val, proc.pid, timerid)?;
		old
	};

	let old_value_val = old_value
		.get_mut(&mut mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	*old_value_val = old;

	Ok(0)
}
