//! The `signal` syscall allows to specify a pointer to a function to be called
//! when a specific signal is received by the current process.

use crate::{
	errno,
	errno::Errno,
	process::{
		signal,
		signal::{SigAction, SigHandler, Signal, SignalHandler},
		Process,
	},
};
use core::{
	ffi::{c_int, c_void},
	mem::transmute,
	ptr::null,
};
use macros::syscall;

#[syscall]
pub fn signal(signum: c_int, handler: *const c_void) -> Result<i32, Errno> {
	if signum < 0 {
		return Err(errno!(EINVAL));
	}
	let signal = Signal::try_from(signum as u32)?;

	let h = match handler {
		signal::SIG_IGN => SignalHandler::Ignore,
		signal::SIG_DFL => SignalHandler::Default,
		_ => {
			let handler_fn = unsafe { transmute::<*const c_void, SigHandler>(handler) };

			SignalHandler::Handler(SigAction {
				sa_handler: Some(handler_fn),
				sa_sigaction: None,
				sa_mask: 0,
				sa_flags: 0,
				sa_restorer: None,
			})
		}
	};

	let old_handler = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let old_handler = proc.get_signal_handler(&signal);
		proc.set_signal_handler(&signal, h);
		old_handler
	};

	let old_handler_ptr = match old_handler {
		SignalHandler::Ignore => signal::SIG_IGN,
		SignalHandler::Default => signal::SIG_DFL,

		SignalHandler::Handler(action) => {
			if let Some(handler) = action.sa_handler {
				handler as *const c_void
			} else {
				null::<c_void>()
			}
		}
	};
	Ok(old_handler_ptr as _)
}
