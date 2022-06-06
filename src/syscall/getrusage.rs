//! The `getrusage` system call returns the system usage for the current process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::process::rusage::RUsage;

/// Returns the resource usage of the current process.
const RUSAGE_SELF: i32 = 0;
/// Returns the resource usage of the process's children.
const RUSAGE_CHILDREN: i32 = -1;

/// The implementation of the `getrusage` syscall.
pub fn getrusage(regs: &Regs) -> Result<i32, Errno> {
	let who = regs.ebx as i32;
	let usage: SyscallPtr<RUsage> = (regs.ecx as usize).into();

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	// TODO Check access to `usage`

	let rusage = match who {
		RUSAGE_SELF => {
			proc.get_rusage().clone()
		},

		RUSAGE_CHILDREN => {
			// TODO Return resources of terminates children
			RUsage::default()
		}

		_ => return Err(errno!(EINVAL)),
	};

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let usage_val = usage.get_mut(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
	*usage_val = rusage;

	Ok(0)
}
