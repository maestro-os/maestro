//! The `timer_create` system call creates a per-process timer.

use crate::{
	errno::Errno,
	process::{
		mem_space::ptr::SyscallPtr,
		signal::{SigEvent, SigVal, Signal, SIGEV_SIGNAL},
		Process,
	},
	time::unit::{ClockIdT, TimerT},
};
use macros::syscall;

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
