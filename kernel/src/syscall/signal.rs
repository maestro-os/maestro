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

use crate::{
	process::{
		signal,
		signal::{SigAction, Signal, SignalHandler, SA_RESTART},
		Process,
	},
	syscall::Args,
};
use core::{
	ffi::{c_int, c_void},
	mem,
	mem::transmute,
	ptr::null,
};
use utils::{
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn signal(
	Args((signum, handler)): Args<(c_int, *const c_void)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Validation
	let signal = Signal::try_from(signum)?;
	// Conversion
	let new_handler = SignalHandler::from_legacy(handler);
	// Set new handler and get old
	let old_handler = mem::replace(
		&mut proc.signal.lock().handlers.lock()[signal as usize],
		new_handler,
	);
	// Convert to pointer and return
	Ok(old_handler.to_legacy() as _)
}
