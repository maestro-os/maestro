//! The `signal` syscall allows to specify a pointer to a function to be called when a specific
//! signal is received by the current process.

use core::ffi::c_void;
use core::mem::transmute;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process::signal::SigHandler;
use crate::process::signal::SignalHandler;
use crate::process::signal;

/// Ignoring the signal.
const SIG_IGN: *const c_void = 0x0 as _;
/// The default action for the signal.
const SIG_DFL: *const c_void = 0x1 as _;

/// The implementation of the `signal` syscall.
pub fn signal(regs: &Regs) -> Result<i32, Errno> {
	let signum = regs.ebx as i32;
	let handler = regs.ecx as *const c_void;

	if signum as usize >= signal::SIGNALS_COUNT {
		return Err(errno::EINVAL);
	}

	let h = match handler {
		SIG_IGN => SignalHandler::Ignore,
		SIG_DFL => SignalHandler::Default,
		_ => {
			let handler_fn = unsafe {
				transmute::<*const c_void, SigHandler>(handler)
			};

			SignalHandler::Handler(handler_fn)
		},
	};

	let old_handler = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let old_handler = proc.get_signal_handler(signum as _);
		proc.set_signal_handler(signum as _, h);
		old_handler
	};

	let old_handler_ptr = match old_handler {
		SignalHandler::Ignore => SIG_IGN,
		SignalHandler::Default => SIG_DFL,
		SignalHandler::Handler(handler) => {
			let handler_ptr = unsafe {
				transmute::<SigHandler, *const c_void>(handler)
			};

			handler_ptr
		},
	};
	Ok(old_handler_ptr as _)
}
