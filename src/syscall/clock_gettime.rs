//! The `clock_gettime` syscall returns the current time of the given clock.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::time::unit::Timespec;
use crate::time;

/// The implementation of the `clock_gettime` syscall.
pub fn clock_gettime(regs: &Regs) -> Result<i32, Errno> {
	let _clock_id = regs.ebx as i32;
	let tp: SyscallPtr<Timespec> = (regs.ecx as usize).into();

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
