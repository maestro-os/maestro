//! The `getrusage` system call returns the system usage for the current process.

use core::ptr::copy_nonoverlapping;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process::rusage::RUsage;

/// Returns the resource usage of the current process.
const RUSAGE_SELF: i32 = 0;
/// Returns the resource usage of the process's children.
const RUSAGE_CHILDREN: i32 = -1;

/// The implementation of the `getrusage` syscall.
pub fn getrusage(regs: &Regs) -> Result<i32, Errno> {
	let who = regs.ebx as i32;
	let usage = regs.ecx as *mut RUsage;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
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

		_ => return Err(errno::EINVAL),
	};

	unsafe { // Safe because the access has been checked before
		copy_nonoverlapping(&rusage, usage, 1);
	}

	Ok(0)
}
