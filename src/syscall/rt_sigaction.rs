//! The `rt_sigaction` system call sets the action for a signal.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::signal::SigAction;
use crate::process::signal::SignalHandler;
use crate::process::Process;
use crate::syscall::Signal;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `rt_sigaction` syscall.
#[syscall]
pub fn rt_sigaction(
	signum: c_int,
	act: SyscallPtr<SigAction>,
	oldact: SyscallPtr<SigAction>,
) -> Result<i32, Errno> {
	let signal = Signal::from_id(signum as _)?;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Save the old structure
	if let Some(oldact) = oldact.get_mut(&mem_space_guard)? {
		let action = proc.get_signal_handler(&signal).get_action();
		*oldact = action;
	}

	// Set the new structure
	if let Some(act) = act.get(&mem_space_guard)? {
		proc.set_signal_handler(&signal, SignalHandler::Handler(*act));
	}

	Ok(0)
}
