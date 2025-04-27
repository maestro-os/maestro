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

//! The `rt_sigaction` system call sets the action for a signal.

use crate::{
	memory::user::UserPtr,
	process::{
		signal::{CompatSigAction, SigAction, SignalHandler},
		Process,
	},
	syscall::{Args, Signal},
};
use core::{ffi::c_int, fmt::Debug};
use utils::{errno::EResult, ptr::arc::Arc};

fn do_rt_sigaction<S: Debug + From<SigAction> + Into<SigAction>>(
	signum: c_int,
	act: UserPtr<S>,
	oldact: UserPtr<S>,
	proc: Arc<Process>,
) -> EResult<usize> {
	let signal = Signal::try_from(signum)?;
	let signal_manager = proc.signal.lock();
	let mut signal_handlers = signal_manager.handlers.lock();
	// Save the old structure
	let old = signal_handlers[signal as usize].get_action().into();
	oldact.copy_to_user(&old)?;
	// Set the new structure
	if let Some(new) = act.copy_from_user()? {
		signal_handlers[signal as usize] = SignalHandler::Handler(new.into());
	}
	Ok(0)
}

pub fn rt_sigaction(
	Args((signum, act, oldact)): Args<(c_int, UserPtr<SigAction>, UserPtr<SigAction>)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	do_rt_sigaction(signum, act, oldact, proc)
}

pub fn compat_rt_sigaction(
	Args((signum, act, oldact)): Args<(c_int, UserPtr<CompatSigAction>, UserPtr<CompatSigAction>)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	do_rt_sigaction(signum, act, oldact, proc)
}
