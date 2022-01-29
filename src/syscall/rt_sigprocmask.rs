//! The rt_sigprocmask system call allows to change the blocked signal mask.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// TODO doc
const SIG_BLOCK: i32 = 0;
/// TODO doc
const SIG_UNBLOCK: i32 = 1;
/// TODO doc
const SIG_SETMASK: i32 = 2;

/// The implementation of the `rt_sigprocmask` syscall.
pub fn rt_sigprocmask(regs: &Regs) -> Result<i32, Errno> {
	let how = regs.ebx as i32;
	let _set = regs.ecx as *const u32;
	let _oldset = regs.edx as *mut u32;
	let _sigsetsize = regs.esi as u32;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let _proc = guard.get_mut();

	// TODO Check access to `set` and `oldset`

	// TODO Get slices

	match how {
		SIG_BLOCK => {
			// TODO
		},

		SIG_UNBLOCK => {
			// TODO
		},

		SIG_SETMASK => {
			// TODO
		},

		_ => return Err(errno::EINVAL),
	}

	Ok(0)
}
