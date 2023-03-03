//! `clock_gettime64` is like `clock_gettime` but using 64 bits.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::time;
use crate::time::unit::Timespec;
use macros::syscall;

// TODO Check first arg type
#[syscall]
pub fn clock_gettime64(_clock_id: i32, tp: SyscallPtr<Timespec>) -> Result<i32, Errno> {
	// TODO Get clock according to param
	let clk = b"TODO";
	let curr_time = time::get_struct::<Timespec>(clk, true).ok_or(errno!(EINVAL))?;

	{
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let timespec = tp.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

		*timespec = curr_time;
	}

	Ok(0)
}
