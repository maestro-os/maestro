//! TODO doc

use core::mem::size_of;
use core::ptr::NonNull;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process;
use crate::util;

/// Wait flag. Returns immediately if no child has exited.
const WNOHANG: i32 =    0b1;
/// Wait flag. Returns if a child has stopped.
const WUNTRACED: i32 =  0b10;
/// Wait flag. Returns if a stopped child has been resumed by delivery of SIGCONT.
const WCONTINUED: i32 = 0b100;

/// Tells whether at least one process matches the given constraint for the syscall.
pub fn any_target(proc: &Process, pid: i32) -> bool {
	if pid < -1 {
		// TODO wait for any child process whose process group ID is equal to the absolute value of
		// pid.
		todo!();
	} else if pid == -1 {
		proc.get_children().len() > 0
	} else if pid == 0 {
		// TODO wait for any child process whose process group ID is equal to that of the calling
		// process at the time of the call to waitpid().
		todo!();
	} else {
		// TODO wait for the child whose process ID is equal to the value of pid.
		todo!();
	}
}

/// The implementation of the `waitpid` syscall.
pub fn waitpid(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let wstatus = regs.ecx as *mut i32;
	let options = regs.edx as i32;

	let wstatus = NonNull::new(wstatus);
	if let Some(mut wstatus) = wstatus {
		let ptr = unsafe {
			wstatus.as_mut() as *mut _ as *mut _
		};

		if !proc.get_mem_space().can_access(ptr, size_of::<i32>(), true, true) {
			return Err(errno::EINVAL);
		}
	}

	if !any_target(proc, pid) {
		return Err(errno::ECHILD);
	}

	// TODO Check now if a target process is waitable
	// TODO If yes, return the PID and write `wstatus` if available

	if options & WNOHANG != 0 {
		// TODO Store waiting options into the process's structure?
		// When a child process is paused or resumed by a signal or is terminated, it changes the
		// state of the current process to wake it up
		proc.set_state(process::State::Sleeping);
		crate::wait();

		// TODO Check again if a target process is waitable
		// TODO If yes, return the PID and write `wstatus` if available
		todo!();
	} else {
		Ok(0)
	}
}
