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

//! POSIX signals implementation.

pub mod ucontext;

use super::{Process, REDZONE_SIZE, State};
use crate::{
	arch::x86::idt::IntFrame,
	file::perm::Uid,
	memory::{VirtAddr, user::UserPtr},
	process,
	process::pid::Pid,
	syscall::{
		FromSyscallArg,
		wait::{WCONTINUED, WEXITED, WUNTRACED},
	},
	time::unit::ClockIdT,
};
use core::{
	ffi::{c_int, c_void},
	hint::{likely, unlikely},
	mem::size_of,
	ptr::NonNull,
};
use ucontext::UContext32;
#[cfg(target_pointer_width = "64")]
use ucontext::UContext64;
use utils::{errno, errno::Errno};

/// sigaltstack flag: Currently executing on the alternate signal stack
pub const SS_ONSTACK: i32 = 1;
/// sigaltstack flag: The alternate signal stack is currently disabled
pub const SS_DISABLE: i32 = 2;
/// sigaltstack flag: Autodisarm on signal handler entry
pub const SS_AUTODISARM: i32 = 1 << 31;

/// Signal handler value: Ignoring the signal.
pub const SIG_IGN: usize = 0x0;
/// Signal handler value: The default action for the signal.
pub const SIG_DFL: usize = 0x1;

// TODO implement all flags
/// [`SigAction`] flag: If set, use `sa_sigaction` instead of `sa_handler`.
pub const SA_SIGINFO: u64 = 0x00000004;
/// [`SigAction`] flag: If set, use [`SigAction::sa_restorer`] as signal trampoline.
pub const SA_RESTORER: u64 = 0x04000000;
/// [`SigAction`] flag: If set, use an alternate stack if available
pub const SA_ONSTACK: u64 = 0x08000000;
/// [`SigAction`] flag: If set, the system call must restart after being interrupted by a signal.
pub const SA_RESTART: u64 = 0x10000000;
/// [`SigAction`] flag: If set, the signal is not added to the signal mask of the process when
/// executed.
pub const SA_NODEFER: u64 = 0x40000000;

/// Notify method: generate a signal
pub const SIGEV_SIGNAL: c_int = 0;
/// Notify method: do nothing
pub const SIGEV_NONE: c_int = 1;
/// Notify method: starts a function as a new thread
pub const SIGEV_THREAD: c_int = 2;

/// The size of the signal handlers table (the number of signals + 1, since
/// indexing begins at 1 instead of 0).
pub const SIGNALS_COUNT: usize = 65;

/// 32-bit version of `stack_t`
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Stack32 {
	/// Stack pointer
	pub ss_sp: u32,
	/// Flags
	pub ss_flags: i32,
	/// Stack size
	pub ss_size: u32,
}

impl Default for Stack32 {
	fn default() -> Self {
		Self {
			ss_sp: 0,
			ss_flags: SS_DISABLE,
			ss_size: 0,
		}
	}
}

impl From<Stack64> for Stack32 {
	fn from(ss: Stack64) -> Self {
		Self {
			ss_sp: ss.ss_sp as _,
			ss_flags: ss.ss_flags,
			ss_size: ss.ss_size as _,
		}
	}
}

/// 64-bit version of `stack_t`
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Stack64 {
	/// Stack pointer
	pub ss_sp: u64,
	/// Flags
	pub ss_flags: i32,
	/// Stack size
	pub ss_size: usize,
}

impl Default for Stack64 {
	fn default() -> Self {
		Self {
			ss_sp: 0,
			ss_flags: SS_DISABLE,
			ss_size: 0,
		}
	}
}

impl From<Stack32> for Stack64 {
	fn from(ss: Stack32) -> Self {
		Self {
			ss_sp: ss.ss_sp as _,
			ss_flags: ss.ss_flags,
			ss_size: ss.ss_size as _,
		}
	}
}

#[cfg(target_pointer_width = "32")]
/// Kernelspace alternative stack structure
pub type AltStack = Stack32;
#[cfg(target_pointer_width = "64")]
/// Kernelspace alternative stack structure
pub type AltStack = Stack64;

/// Enumeration representing the action to perform for a signal.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SignalAction {
	/// Abnormal termination of the process.
	Terminate,
	/// Abnormal termination of the process with additional actions.
	Abort,
	/// Ignore the signal.
	Ignore,
	/// Stop the process.
	Stop,
	/// Continue the process, if it is stopped; otherwise, ignore the signal.
	Continue,
}

impl SignalAction {
	/// Executes the signal action for the current process.
	pub fn exec(self, sig: Signal) {
		let proc = Process::current();
		match self {
			// TODO when `Abort`ing, dump core
			SignalAction::Terminate | SignalAction::Abort => {
				proc.signal.lock().termsig = sig.0 as u8;
				process::set_state(State::Zombie);
				proc.notify_parent(WEXITED as u8);
			}
			SignalAction::Ignore => {}
			SignalAction::Stop => {
				proc.signal.lock().termsig = sig.0 as u8;
				process::set_state(State::Stopped);
				proc.notify_parent(WUNTRACED as u8);
			}
			SignalAction::Continue => proc.notify_parent(WCONTINUED as u8),
		}
	}
}

/// A signal handler value.
pub type SigVal = usize;

// FIXME: fields are incorrect (check musl source)
/// Signal information.
#[repr(C)]
#[derive(Debug)]
pub struct SigInfo {
	/// Signal number.
	pub si_signo: i32,
	/// An errno value.
	si_errno: i32,
	/// Signal code.
	si_code: i32,
	/// Trap number that caused hardware-generated signal.
	si_trapno: i32,
	/// Sending process ID.
	si_pid: Pid,
	/// Real user ID of sending process.
	si_uid: Uid,
	/// Exit value or signal.
	si_status: i32,
	/// User time consumed.
	si_utime: ClockIdT,
	/// System time consumed.
	si_stime: ClockIdT,
	/// Signal value
	si_value: SigVal,
	/// POSIX.1b signal.
	si_int: i32,
	/// POSIX.1b signal.
	si_ptr: *mut c_void,
	/// Timer overrun count.
	si_overrun: i32,
	/// Timer ID.
	si_timerid: i32,
	/// Memory location which caused fault.
	si_addr: *mut c_void,
	/// Band event.
	si_band: i32, // FIXME long (64bits?)
	/// File descriptor.
	si_fd: i32,
	/// Least significant bit of address.
	si_addr_lsb: i16,
	/// Lower bound when address violation.
	si_lower: *mut c_void,
	/// Upper bound when address violation.
	si_upper: *mut c_void,
	/// Protection key on PTE that caused fault.
	si_pkey: i32,
	/// Address of system call instruction.
	si_call_addr: *mut c_void,
	/// Number of attempted system call.
	si_syscall: i32,
	/// Architecture of attempted system call.
	si_arch: u32,
}

/// Kernelspace signal mask.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SigSet(pub u64);

impl SigSet {
	/// Tells whether the set is all cleared.
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.0 == 0
	}

	/// Tells whether the `n`th bit is set.
	#[inline]
	pub fn is_set(&self, n: usize) -> bool {
		self.0 & (1u64 << n) != 0
	}

	/// Sets the `n`th bit.
	#[inline]
	pub fn set(&mut self, n: usize) {
		self.0 |= 1u64 << n;
	}

	/// Sets the `n`th bit.
	#[inline]
	pub fn clear(&mut self, n: usize) {
		self.0 &= !(1u64 << n);
	}

	/// Returns an iterator over the bitset's values
	#[inline]
	pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
		(0..64).map(|n| self.is_set(n))
	}
}

/// Action to be executed when a signal is received.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SigAction {
	/// Pointer to the signal handler.
	pub sa_handler: usize,
	/// A set of flags which modifies the behaviour of the signal.
	pub sa_flags: u64,
	/// Pointer to the signal trampoline.
	pub sa_restorer: usize,
	/// A mask of signals that should be masked while executing the signal
	/// handler.
	pub sa_mask: SigSet,
}

impl From<CompatSigAction> for SigAction {
	fn from(sig_action: CompatSigAction) -> Self {
		let sa_mask = sig_action.sa_mask[0] as u64 | ((sig_action.sa_mask[1] as u64) << 32);
		Self {
			sa_handler: sig_action.sa_handler as _,
			sa_flags: sig_action.sa_flags as _,
			sa_restorer: sig_action.sa_restorer as _,
			sa_mask: SigSet(sa_mask),
		}
	}
}

/// Compatibility version of [`SigAction`].
#[allow(missing_docs)]
#[repr(C)]
#[derive(Clone, Debug)]
pub struct CompatSigAction {
	pub sa_handler: u32,
	pub sa_flags: u32,
	pub sa_restorer: u32,
	pub sa_mask: [u32; 2],
}

impl From<SigAction> for CompatSigAction {
	fn from(sig_action: SigAction) -> Self {
		let sa_mask = [
			sig_action.sa_mask.0 as u32,
			(sig_action.sa_mask.0 >> 32) as u32,
		];
		Self {
			sa_handler: sig_action.sa_handler as _,
			sa_flags: sig_action.sa_flags as _,
			sa_restorer: sig_action.sa_restorer as _,
			sa_mask,
		}
	}
}

/// Notification from asynchronous routines.
#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct SigEvent {
	/// Notification method.
	pub sigev_notify: c_int,
	/// Notification signal.
	pub sigev_signo: c_int,
	/// TODO doc
	pub sigev_value: SigVal,
	/// Function used for thread notification.
	pub sigev_notify_function: Option<NonNull<extern "C" fn(SigVal)>>,
	/// Data passed with notification.
	pub sigev_notify_attributes: Option<NonNull<c_void>>,
	/// ID of thread to signal.
	pub sigev_notify_thread_id: Pid,
}

impl SigEvent {
	/// Tells whether the structure is valid.
	pub fn is_valid(&self) -> bool {
		// TODO check sigev_notify_thread_id
		match self.sigev_notify {
			SIGEV_NONE | SIGEV_THREAD => true,
			SIGEV_SIGNAL => Signal::try_from(self.sigev_signo).is_ok(),
			_ => false,
		}
	}
}

/// Enumeration containing the different possibilities for signal handling.
#[derive(Clone, Debug, Default)]
pub enum SignalHandler {
	/// Ignores the signal.
	Ignore,
	/// Executes the default action.
	#[default]
	Default,
	/// A custom action defined with a call to signal.
	Handler(SigAction),
}

impl From<SigAction> for SignalHandler {
	fn from(action: SigAction) -> Self {
		match action.sa_handler {
			SIG_IGN => Self::Ignore,
			SIG_DFL => Self::Default,
			_ => Self::Handler(action),
		}
	}
}

impl SignalHandler {
	/// Creates a handler from a value given by the `signal` system call.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn from_legacy(handler: *const c_void) -> Self {
		match handler as usize {
			SIG_IGN => Self::Ignore,
			SIG_DFL => Self::Default,
			handler => Self::Handler(SigAction {
				sa_handler: handler,
				// TODO use System V semantic like Linux instead of BSD? (SA_RESETHAND |
				// SA_NODEFER)
				sa_flags: SA_RESTART,
				sa_restorer: 0,
				sa_mask: Default::default(),
			}),
		}
	}

	/// The opposite operation of [`Self::from_legacy`].
	pub fn to_legacy(&self) -> usize {
		match self {
			SignalHandler::Ignore => SIG_IGN,
			SignalHandler::Default => SIG_DFL,
			SignalHandler::Handler(action) => action.sa_handler as _,
		}
	}

	/// Returns an instance of [`SigAction`] associated with the handler.
	pub fn get_action(&self) -> SigAction {
		match self {
			Self::Ignore => SigAction {
				sa_handler: SIG_IGN,
				sa_flags: 0,
				sa_restorer: 0,
				sa_mask: Default::default(),
			},
			Self::Default => SigAction {
				sa_handler: SIG_DFL,
				sa_flags: 0,
				sa_restorer: 0,
				sa_mask: Default::default(),
			},
			Self::Handler(action) => *action,
		}
	}

	/// Executes the action for `signal` on the current process.
	pub fn exec(&self, signal: Signal, frame: &mut IntFrame) {
		let proc = Process::current();
		let action = match self {
			Self::Handler(action) if signal.can_catch() => action,
			Self::Ignore => return,
			// Execute default action
			_ => {
				// Signals on the init process can be executed only if the process has set a
				// signal handler
				if !proc.is_init() || !signal.can_catch() {
					signal.get_default_action().exec(signal);
				}
				return;
			}
		};
		// TODO handle SA_SIGINFO
		// Prepare the signal handler stack
		let (stack_addr, altstack, sigmask) = {
			let mut sig = proc.signal.lock();
			let altstack = sig.altstack.clone();
			let stack_addr = if action.sa_flags & SA_ONSTACK != 0
				&& sig.altstack.ss_flags & SS_DISABLE == 0
				&& likely(sig.altstack.ss_sp != 0)
			{
				sig.altstack.ss_flags |= SS_ONSTACK;
				if sig.altstack.ss_flags & SS_AUTODISARM != 0 {
					sig.altstack = Default::default();
				}
				VirtAddr(altstack.ss_sp as _)
			} else {
				VirtAddr(frame.get_stack_address().saturating_sub(REDZONE_SIZE))
			};
			(stack_addr, altstack, sig.sigmask)
		};
		// Size of the `ucontext_t` struct and arguments *on the stack*
		let (ctx_size, ctx_align) = if frame.is_compat() {
			(size_of::<UContext32>(), align_of::<UContext32>())
		} else {
			#[cfg(target_pointer_width = "32")]
			unreachable!();
			#[cfg(target_pointer_width = "64")]
			(size_of::<UContext64>(), align_of::<UContext64>())
		};
		let ctx_addr = VirtAddr(stack_addr.saturating_sub(ctx_size)).down_align_to(ctx_align);
		let signal_sp = VirtAddr(ctx_addr.saturating_sub(size_of::<u64>()));
		// Write data on stack
		if frame.is_compat() {
			let ctx = UContext32::new(altstack.into(), sigmask, frame);
			let res = UserPtr::<UContext32>::from_ptr(ctx_addr.0).copy_to_user(&ctx);
			if unlikely(res.is_err()) {
				Signal::SIGSEGV.get_default_action().exec(Signal::SIGSEGV);
				return;
			}
			let res = UserPtr::<[u32; 2]>::from_ptr(signal_sp.0).copy_to_user(&[
				// Return pointer
				action.sa_restorer as _,
				// Argument
				signal.0 as _,
			]);
			if unlikely(res.is_err()) {
				Signal::SIGSEGV.get_default_action().exec(Signal::SIGSEGV);
				return;
			}
		} else {
			#[cfg(target_pointer_width = "64")]
			{
				let ctx = UContext64::new(altstack, sigmask, frame);
				let res = UserPtr::<UContext64>::from_ptr(ctx_addr.0).copy_to_user(&ctx);
				if unlikely(res.is_err()) {
					Signal::SIGSEGV.get_default_action().exec(Signal::SIGSEGV);
					return;
				}
				// Return pointer
				let res =
					UserPtr::<u64>::from_ptr(signal_sp.0).copy_to_user(&(action.sa_restorer as _));
				if unlikely(res.is_err()) {
					Signal::SIGSEGV.get_default_action().exec(Signal::SIGSEGV);
					return;
				}
			}
		}
		// Block signal from `sa_mask`
		{
			let mut signals_manager = proc.signal.lock();
			signals_manager.sigmask.0 |= action.sa_mask.0;
			if action.sa_flags & SA_NODEFER == 0 {
				signals_manager.sigmask.set(signal.0 as usize);
			}
		}
		// Prepare registers for the trampoline
		frame.rbp = 0;
		frame.rsp = signal_sp.0 as _;
		frame.rip = action.sa_handler as _;
		#[cfg(target_pointer_width = "64")]
		if !frame.is_compat() {
			frame.rcx = frame.rip;
			// Argument
			frame.rdi = signal.0 as _;
		}
	}
}

/// A POSIX signal.
///
/// This is a thin wrapper around the raw signal number so that POSIX realtime signals (32
/// through `SIGRTMAX`) — which are valid but don't have a named identifier — round-trip
/// through the kernel without requiring an enum variant per number.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Signal(pub i32);

#[rustfmt::skip]
impl Signal {
	/// Hangup.
	pub const SIGHUP: Self = Self(1);
	/// Terminal interrupt.
	pub const SIGINT: Self = Self(2);
	/// Terminal quit.
	pub const SIGQUIT: Self = Self(3);
	/// Illegal instruction.
	pub const SIGILL: Self = Self(4);
	/// Trace/breakpoint trap.
	pub const SIGTRAP: Self = Self(5);
	/// Process abort.
	pub const SIGABRT: Self = Self(6);
	/// Access to an undefined portion of a memory object.
	pub const SIGBUS: Self = Self(7);
	/// Erroneous arithmetic operation.
	pub const SIGFPE: Self = Self(8);
	/// Kill.
	pub const SIGKILL: Self = Self(9);
	/// User-defined signal 1.
	pub const SIGUSR1: Self = Self(10);
	/// Invalid memory reference.
	pub const SIGSEGV: Self = Self(11);
	/// User-defined signal 2.
	pub const SIGUSR2: Self = Self(12);
	/// Write on a pipe with no one to read it.
	pub const SIGPIPE: Self = Self(13);
	/// Alarm clock.
	pub const SIGALRM: Self = Self(14);
	/// Termination.
	pub const SIGTERM: Self = Self(15);
	/// Child process terminated.
	pub const SIGCHLD: Self = Self(17);
	/// Continue executing.
	pub const SIGCONT: Self = Self(18);
	/// Stop executing.
	pub const SIGSTOP: Self = Self(19);
	/// Terminal stop.
	pub const SIGTSTP: Self = Self(20);
	/// Background process attempting to read.
	pub const SIGTTIN: Self = Self(21);
	/// Background process attempting to write.
	pub const SIGTTOU: Self = Self(22);
	/// High bandwidth data is available at a socket.
	pub const SIGURG: Self = Self(23);
	/// CPU time limit exceeded.
	pub const SIGXCPU: Self = Self(24);
	/// File size limit exceeded.
	pub const SIGXFSZ: Self = Self(25);
	/// Virtual timer expired.
	pub const SIGVTALRM: Self = Self(26);
	/// Profiling timer expired.
	pub const SIGPROF: Self = Self(27);
	/// Window resize.
	pub const SIGWINCH: Self = Self(28);
	/// Pollable event.
	pub const SIGPOLL: Self = Self(29);
	/// Bad system call.
	pub const SIGSYS: Self = Self(31);

	/// First POSIX realtime signal.
	pub const SIGRTMIN: Self = Self(32);
	/// Last POSIX realtime signal.
	pub const SIGRTMAX: Self = Self(64);
}

impl Signal {
	/// Returns the default action for the signal.
	pub fn get_default_action(self) -> SignalAction {
		match self.0 {
			1 | 2 => SignalAction::Terminate,
			3 => SignalAction::Abort,
			4..=8 => SignalAction::Abort,
			9 => SignalAction::Terminate,
			10 => SignalAction::Terminate,
			11 => SignalAction::Abort,
			12..=15 => SignalAction::Terminate,
			17 => SignalAction::Ignore,
			18 => SignalAction::Continue,
			19..=22 => SignalAction::Stop,
			23 => SignalAction::Ignore,
			24 | 25 => SignalAction::Abort,
			26 | 27 => SignalAction::Terminate,
			28 => SignalAction::Ignore,
			29 => SignalAction::Terminate,
			31 => SignalAction::Abort,
			32..=64 => SignalAction::Terminate,
			_ => SignalAction::Terminate,
		}
	}

	/// Tells whether the signal can be caught.
	pub fn can_catch(self) -> bool {
		!matches!(self.0, 9 | 11 | 19 | 31) // SIGKILL, SIGSEGV, SIGSTOP, SIGSYS
	}
}

impl TryFrom<i32> for Signal {
	type Error = Errno;

	fn try_from(id: i32) -> Result<Self, Self::Error> {
		// Standard signals (1-15, 17-29, 31) plus POSIX realtime signals (32-64).
		if matches!(id, 1..=15 | 17..=29 | 31 | 32..=64) {
			Ok(Self(id))
		} else {
			Err(errno!(EINVAL))
		}
	}
}
