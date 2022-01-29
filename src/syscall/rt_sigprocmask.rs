//! The rt_sigprocmask system call allows to change the blocked signal mask.

use core::cmp::min;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

/// The implementation of the `rt_sigprocmask` syscall.
pub fn rt_sigprocmask(regs: &Regs) -> Result<i32, Errno> {
	let how = regs.ebx as i32;
	let set = regs.ecx as *const u8;
	let oldset = regs.edx as *mut u8;
	let sigsetsize = regs.esi as u32;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// TODO Check access to `set` and `oldset`

	// Getting slices to pointers
	let set_slice = if !set.is_null() {
		Some(unsafe { // Safe because access has been checked before
			slice::from_raw_parts(set, sigsetsize as _)
		})
	} else {
		None
	};
	let oldset_slice = if !oldset.is_null() {
		Some(unsafe { // Safe because access has been checked before
			slice::from_raw_parts_mut(oldset, sigsetsize as _)
		})
	} else {
		None
	};

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

			_ => return Err(errno::EINVAL),
		}
	}

	Ok(0)
}
