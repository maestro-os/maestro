//! The `timer_delete` system call deletes a per-process timer.

use crate::errno::Errno;
use crate::process::Process;
use crate::time::unit::TimerT;
use macros::syscall;

#[syscall]
pub fn timer_delete(timerid: TimerT) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	proc.timer_manager().lock().delete_timer(timerid)?;
	Ok(0)
}
