//! The rt_sigprocmask system call allows to change the blocked signal mask.

use core::cmp::min;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

// TODO Use SigSet in crate::process::signal
/// The implementation of the `rt_sigprocmask` syscall.
pub fn rt_sigprocmask(regs: &Regs) -> Result<i32, Errno> {
	let how = regs.ebx as i32;
	let set: SyscallSlice<u8> = (regs.ecx as usize).into();
	let oldset: SyscallSlice<u8> = (regs.edx as usize).into();
	let sigsetsize = regs.esi as u32;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Getting slices
	let set_slice = set.get(&mem_space_guard, sigsetsize as _)?;
	let oldset_slice = oldset.get_mut(&mem_space_guard, sigsetsize as _)?;

	// The current set
	let curr = proc.get_sigmask_mut();

	if let Some(oldset) = oldset_slice {
		// Saving the old set
		for i in 0..min(oldset.len(), curr.len()) {
			oldset[i] = curr[i];
		}
	}

	if let Some(set) = set_slice {
		// Applies the operation
		match how {
			SIG_BLOCK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] |= set[i];
				}
			},

			SIG_UNBLOCK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] &= !set[i];
				}
			},

			SIG_SETMASK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] = set[i];
				}
			},

			_ => return Err(errno!(EINVAL)),
		}
	}

	Ok(0)
}
