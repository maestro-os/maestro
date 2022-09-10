//! `clock_gettime64` is like `clock_gettime` but using 64 bits.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::process::Process;
use crate::time;
use crate::time::unit::Timespec;

/// The implementation of the `clock_gettime64` syscall.
pub fn clock_gettime64(regs: &Regs) -> Result<i32, Errno> {
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

		crate::println!("time64: {:?}", curr_time); // TODO rm
		*timespec = curr_time;
	}

	Ok(0)
}
