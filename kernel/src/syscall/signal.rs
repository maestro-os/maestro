/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The `signal` syscall allows to specify a pointer to a function to be called
//! when a specific signal is received by the current process.

use crate::process::{
	signal,
	signal::{SigAction, Signal, SignalHandler, SA_RESTART},
	Process,
};
use core::{
	ffi::{c_int, c_void},
	mem,
	mem::transmute,
	ptr::null,
};
use macros::syscall;
use utils::errno::Errno;

#[syscall]
pub fn signal(signum: c_int, handler: *const c_void) -> Result<i32, Errno> {
	// Validation
	let signal = Signal::try_from(signum as u32)?;
	// Conversion
	let new_handler = match handler {
		signal::SIG_IGN => SignalHandler::Ignore,
		signal::SIG_DFL => SignalHandler::Default,
		_ => SignalHandler::Handler(SigAction {
			sa_handler: Some(unsafe { transmute(handler) }),
			sa_sigaction: None,
			sa_mask: 0,
			sa_flags: SA_RESTART,
		}),
	};
	// Set new handler and get old
	let old_handler = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		let mut signal_handlers = proc.signal_handlers.lock();
		mem::replace(&mut signal_handlers[signal.get_id() as usize], new_handler)
	};
	// Convert to pointer and return
	let ptr = match old_handler {
		SignalHandler::Ignore => signal::SIG_IGN,
		SignalHandler::Default => signal::SIG_DFL,
		SignalHandler::Handler(action) => action
			.sa_handler
			.map(|ptr| ptr as *const c_void)
			.unwrap_or(null()),
	};
	Ok(ptr as _)
}
