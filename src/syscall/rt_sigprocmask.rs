//! The rt_sigprocmask system call allows to change the blocked signal mask.

use crate::errno;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use core::cmp::min;
use core::ffi::c_int;
use macros::syscall;

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

// TODO Use SigSet in crate::process::signal
#[syscall]
pub fn rt_sigprocmask(
	how: c_int,
	set: SyscallSlice<u8>,
	oldset: SyscallSlice<u8>,
	sigsetsize: usize,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	let curr = proc.sigmask.as_slice_mut();

	let oldset_slice = oldset.get_mut(&mut mem_space_guard, sigsetsize as _)?;
	if let Some(oldset) = oldset_slice {
		// Saving the old set
		for i in 0..min(oldset.len(), curr.len()) {
			oldset[i] = curr[i];
		}
	}

	let set_slice = set.get(&mem_space_guard, sigsetsize as _)?;
	if let Some(set) = set_slice {
		// Applies the operation
		match how {
			SIG_BLOCK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] |= set[i];
				}
			}

			SIG_UNBLOCK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] &= !set[i];
				}
			}

			SIG_SETMASK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] = set[i];
				}
			}

			_ => return Err(errno!(EINVAL)),
		}
	}

	Ok(0)
}
