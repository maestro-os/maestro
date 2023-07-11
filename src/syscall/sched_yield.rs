//! The `sched_yield` system call ends the current tick of the current process and returns the
//! control back to the scheduler.

use crate::errno::Errno;
use crate::process::scheduler;
use macros::syscall;

#[syscall]
pub fn sched_yield() -> Result<i32, Errno> {
	scheduler::end_tick();
	Ok(0)
}
