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
	arch::x86::idt::IntFrame,
	file::perm::AccessProfile,
	memory::user::UserPtr,
	process,
	process::{
		Process, State,
		pid::Pid,
		scheduler::SCHEDULER,
		signal,
		signal::{CompatSigAction, SigAction, SigSet, Signal, SignalHandler, ucontext},
	},
	syscall::{Args, FromSyscallArg},
};
use core::{
	ffi::{c_int, c_void},
	fmt::Debug,
	intrinsics::unlikely,
	mem,
	mem::transmute,
	ptr::null,
};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

pub fn signal(
	Args((signum, handler)): Args<(c_int, *const c_void)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	let signal = Signal::try_from(signum)?;
	let new_handler = SignalHandler::from_legacy(handler);
	let old_handler = mem::replace(
		&mut proc.signal.lock().handlers.lock()[signal as usize],
		new_handler,
	);
	Ok(old_handler.to_legacy() as _)
}

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

pub fn rt_sigprocmask(
	Args((how, set, oldset, sigsetsize)): Args<(c_int, UserPtr<SigSet>, UserPtr<SigSet>, usize)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Validation
	if unlikely(sigsetsize != size_of::<SigSet>()) {
		return Err(errno!(EINVAL));
	}
	let mut signal_manager = proc.signal.lock();
	// Save old set
	oldset.copy_to_user(&signal_manager.sigmask)?;
	// Apply new set
	if let Some(set) = set.copy_from_user()? {
		match how {
			SIG_BLOCK => signal_manager.sigmask.0 |= set.0,
			SIG_UNBLOCK => signal_manager.sigmask.0 &= !set.0,
			SIG_SETMASK => signal_manager.sigmask.0 = set.0,
			_ => return Err(errno!(EINVAL)),
		}
	}
	Ok(0)
}

pub fn sigreturn(frame: &mut IntFrame) -> EResult<usize> {
	let proc = Process::current();
	// Retrieve and restore previous state
	let stack_ptr = frame.get_stack_address();
	if frame.is_compat() {
		let ctx = UserPtr::<ucontext::UContext32>::from_ptr(stack_ptr)
			.copy_from_user()?
			.ok_or_else(|| errno!(EFAULT))?;
		ctx.restore_regs(&proc, frame);
	} else {
		#[cfg(target_arch = "x86_64")]
		{
			let ctx = UserPtr::<ucontext::UContext64>::from_ptr(stack_ptr)
				.copy_from_user()?
				.ok_or_else(|| errno!(EFAULT))?;
			let res = ctx.restore_regs(&proc, frame);
			if unlikely(res.is_err()) {
				proc.kill(Signal::SIGSEGV);
			}
		}
	}
	// Left register untouched
	Ok(frame.get_syscall_id())
}

pub fn rt_sigreturn(frame: &mut IntFrame) -> EResult<usize> {
	sigreturn(frame)
}

/// Tries to kill the process with PID `pid` with the signal `sig`.
///
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn try_kill(pid: Pid, sig: Option<Signal>) -> EResult<()> {
	let proc = Process::current();
	let ap = proc.fs.lock().access_profile;
	// Closure sending the signal
	let f = |target: &Process| {
		if matches!(target.get_state(), State::Zombie) {
			return Ok(());
		}
		if !ap.can_kill(target) {
			return Err(errno!(EPERM));
		}
		if let Some(sig) = sig {
			target.kill(sig);
		}
		Ok(())
	};
	if pid == proc.get_pid() {
		f(&proc)?;
	} else {
		let target_proc = Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?;
		f(&target_proc)?;
	}
	Ok(())
}

/// Tries to kill a process group.
///
/// Arguments:
/// - `pid` is the value that determine which process(es) to kill.
/// - `sig` is the signal to send.
///
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn try_kill_group(pid: i32, sig: Option<Signal>) -> EResult<()> {
	let pgid = match pid {
		0 => Process::current().get_pgid(),
		i if i < 0 => -pid as Pid,
		_ => pid as Pid,
	};
	// Kill process group
	Process::get_by_pid(pgid)
		.ok_or_else(|| errno!(ESRCH))?
		.links
		.lock()
		.process_group
		.iter()
		.try_for_each(|pid| try_kill(*pid as _, sig))
}

pub fn kill(Args((pid, sig)): Args<(c_int, c_int)>) -> EResult<usize> {
	let sig = (sig != 0).then(|| Signal::try_from(sig)).transpose()?;
	match pid {
		// Kill the process with the given PID
		1.. => try_kill(pid as _, sig)?,
		// Kill all processes in the current process group
		0 => try_kill_group(0, sig)?,
		// Kill all processes for which the current process has the permission
		-1 => {
			let sched = SCHEDULER.lock();
			for (pid, _) in sched.iter_process() {
				if *pid == process::pid::INIT_PID {
					continue;
				}
				// TODO Check permission
				try_kill(*pid, sig)?;
			}
		}
		// Kill the given process group
		..-1 => try_kill_group(-pid as _, sig)?,
	}
	Ok(0)
}

pub fn tkill(
	Args((tid, sig)): Args<(Pid, c_int)>,
	access_profile: AccessProfile,
) -> EResult<usize> {
	let signal = Signal::try_from(sig)?;
	let thread = Process::get_by_tid(tid).ok_or(errno!(ESRCH))?;
	if !access_profile.can_kill(&thread) {
		return Err(errno!(EPERM));
	}
	thread.kill(signal);
	Ok(0)
}
