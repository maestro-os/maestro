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

mod trampoline;
pub mod ucontext;

use super::{oom, Process, State, REDZONE_SIZE};
use crate::{
	arch::x86::idt::IntFrame, file::perm::Uid, memory::VirtAddr, process::pid::Pid,
	time::unit::ClockIdT,
};
use core::{
	ffi::{c_int, c_void},
	fmt,
	mem::{size_of, transmute},
	ptr,
	ptr::NonNull,
	slice,
};
use utils::{errno, errno::Errno};

/// Signal handler value: Ignoring the signal.
pub const SIG_IGN: usize = 0x0;
/// Signal handler value: The default action for the signal.
pub const SIG_DFL: usize = 0x1;

// TODO implement all flags
/// [`SigAction`] flag: If set, use `sa_sigaction` instead of `sa_handler`.
pub const SA_SIGINFO: i32 = 0x00000004;
/// [`SigAction`] flag: If set, the system call must restart after being interrupted by a signal.
pub const SA_RESTART: i32 = 0x10000000;
/// [`SigAction`] flag: If set, the signal is not added to the signal mask of the process when
/// executed.
pub const SA_NODEFER: i32 = 0x40000000;

/// Notify method: generate a signal
pub const SIGEV_SIGNAL: c_int = 0;
/// Notify method: do nothing
pub const SIGEV_NONE: c_int = 1;
/// Notify method: starts a function as a new thread
pub const SIGEV_THREAD: c_int = 2;

/// The size of the signal handlers table (the number of signals + 1, since
/// indexing begins at 1 instead of 0).
pub const SIGNALS_COUNT: usize = 32;

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
	/// Executes the signal action for the given process.
	pub fn exec(self, sig: Signal, process: &Process) {
		match self {
			// TODO when `Abort`ing, dump core
			SignalAction::Terminate | SignalAction::Abort => {
				#[cfg(feature = "strace")]
				println!(
					"[strace {pid}] killed by signal `{signal}`",
					pid = process.get_pid(),
					signal = sig.get_id()
				);
				process.set_state(State::Zombie);
			}
			SignalAction::Ignore => {}
			SignalAction::Stop => process.set_state(State::Stopped),
			SignalAction::Continue => process.set_state(State::Running),
		}
	}
}

/// A signal handler value.
pub type SigVal = usize;

// FIXME: fields are incorrect (check musl source)
/// Signal information.
#[repr(C)]
pub struct SigInfo {
	/// Signal number.
	si_signo: i32,
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

/// A bits signal mask.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SigSet(pub u64);

impl SigSet {
	/// Tells whether the `n`th bit is set.
	pub fn is_set(&self, n: usize) -> bool {
		self.0 & (1 << n) != 0
	}

	/// Sets the `n`th bit.
	pub fn set(&mut self, n: usize) {
		self.0 |= (1 << n) as u64;
	}

	/// Sets the `n`th bit.
	pub fn clear(&mut self, n: usize) {
		self.0 &= !((1 << n) as u64);
	}

	/// Returns an iterator over the bitset's values
	pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
		(0..64).map(|n| self.is_set(n))
	}
}

/// Union of the `sa_handler` and `sa_sigaction` fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub union SigActionHandler {
	/// The pointer to the signal's handler.
	pub sa_handler: Option<extern "C" fn(i32)>,
	/// Used instead of `sa_handler` if [`SA_SIGINFO`] is specified in `sa_flags`.
	pub sa_sigaction: Option<extern "C" fn(i32, *mut SigInfo, *mut c_void)>,
}

impl fmt::Debug for SigActionHandler {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ptr = unsafe { self.sa_handler };
		fmt::Debug::fmt(&ptr, f)
	}
}

/// An action to be executed when a signal is received.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SigAction {
	/// The pointer to the signal's handler.
	pub sa_handler: SigActionHandler,
	/// A mask of signals that should be masked while executing the signal
	/// handler.
	pub sa_mask: SigSet,
	/// A set of flags which modifies the behaviour of the signal.
	pub sa_flags: i32,
}

/// Notification from asynchronous routines.
#[repr(C)]
#[derive(Clone, Debug)]
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
		if !matches!(self.sigev_notify, SIGEV_SIGNAL | SIGEV_NONE | SIGEV_THREAD) {
			return false;
		}
		if Signal::try_from(self.sigev_signo).is_err() {
			return false;
		}
		// TODO check sigev_notify_thread_id

		true
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
		let handler = unsafe { transmute::<SigActionHandler, usize>(action.sa_handler) };
		match handler {
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
			_ => Self::Handler(SigAction {
				sa_handler: unsafe { transmute::<*const c_void, SigActionHandler>(handler) },
				sa_mask: Default::default(),
				// TODO use System V semantic like Linux instead of BSD? (SA_RESETHAND |
				// SA_NODEFER)
				sa_flags: SA_RESTART,
			}),
		}
	}

	/// The opposite operation of [`Self::from_legacy`].
	pub fn to_legacy(&self) -> usize {
		match self {
			SignalHandler::Ignore => SIG_IGN,
			SignalHandler::Default => SIG_DFL,
			SignalHandler::Handler(action) => unsafe {
				transmute::<Option<extern "C" fn(i32)>, usize>(action.sa_handler.sa_handler)
			},
		}
	}

	/// Returns an instance of [`SigAction`] associated with the handler.
	pub fn get_action(&self) -> SigAction {
		match self {
			Self::Ignore => SigAction {
				sa_handler: unsafe { transmute::<usize, SigActionHandler>(SIG_IGN) },
				sa_mask: Default::default(),
				sa_flags: 0,
			},
			Self::Default => SigAction {
				sa_handler: unsafe { transmute::<usize, SigActionHandler>(SIG_DFL) },
				sa_mask: Default::default(),
				sa_flags: 0,
			},
			Self::Handler(action) => *action,
		}
	}

	/// Executes the action for `signal` on `process`.
	pub fn exec(&self, signal: Signal, process: &Process, frame: &mut IntFrame) {
		let process_state = process.get_state();
		if matches!(process_state, State::Zombie) {
			return;
		}
		let action = match self {
			Self::Handler(action) if signal.can_catch() => action,
			Self::Ignore => return,
			// Execute default action
			_ => {
				// Signals on the init process can be executed only if the process has set a
				// signal handler
				if !process.is_init() || !signal.can_catch() {
					signal.get_default_action().exec(signal, process);
				}
				return;
			}
		};
		// TODO handle SA_SIGINFO
		// TODO Handle the case where an alternate stack is specified (sigaltstack + flag
		// SA_ONSTACK)
		// Prepare the signal handler stack
		let stack_addr = VirtAddr(frame.get_stack_address()) - REDZONE_SIZE;
		// Size of the `ucontext_t` struct and arguments *on the stack*
		let (ctx_size, arg_len) = if frame.is_32bit() {
			(size_of::<ucontext::UContext32>(), size_of::<usize>() * 4)
		} else {
			#[cfg(target_arch = "x86")]
			unreachable!();
			#[cfg(target_arch = "x86_64")]
			(size_of::<ucontext::UContext64>(), 0)
		};
		let ctx_addr = stack_addr - ctx_size;
		let signal_sp = stack_addr - (ctx_size + arg_len);
		{
			let mem_space = process.get_mem_space().unwrap();
			let mut mem_space = mem_space.lock();
			mem_space.bind();
			// FIXME: a stack overflow would cause an infinite loop
			oom::wrap(|| mem_space.alloc(signal_sp, arg_len));
		}
		let handler_pointer = unsafe { action.sa_handler.sa_handler.unwrap() };
		// Write data on stack
		if frame.is_32bit() {
			let args = unsafe {
				ptr::write_volatile(ctx_addr.as_ptr(), ucontext::UContext32::new(process, frame));
				// Arguments slice
				slice::from_raw_parts_mut(signal_sp.as_ptr::<u32>(), 4)
			};
			// Pointer to  `ctx`
			args[3] = ctx_addr.0 as _;
			// Signal number
			args[2] = signal.get_id() as _;
			// Pointer to the handler
			args[1] = handler_pointer as _;
			// Padding (return pointer)
			args[0] = 0;
		} else {
			#[cfg(target_arch = "x86_64")]
			unsafe {
				ptr::write_volatile(ctx_addr.as_ptr(), ucontext::UContext64::new(process, frame));
			}
		}
		// Block signal from `sa_mask`
		{
			let mut signals_manager = process.signal.lock();
			signals_manager.sigmask.0 |= action.sa_mask.0;
			if action.sa_flags & SA_NODEFER == 0 {
				signals_manager.sigmask.set(signal.get_id() as _);
			}
		}
		// Prepare registers for the trampoline
		frame.rbp = 0;
		frame.rsp = signal_sp.0 as _;
		if frame.is_32bit() {
			frame.rip = trampoline::trampoline32 as *const c_void as _;
		} else {
			#[cfg(target_arch = "x86_64")]
			{
				frame.rip = trampoline::trampoline64 as *const c_void as _;
				// Arguments
				frame.rdi = ctx_addr.0 as _;
				frame.rsi = signal.get_id() as _;
				frame.rdx = handler_pointer as _;
			}
		}
	}
}

/// Enumeration of signal types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Signal {
	/// Hangup.
	SIGHUP,
	/// Terminal interrupt.
	SIGINT,
	/// Terminal quit.
	SIGQUIT,
	/// Illegal instruction.
	SIGILL,
	/// Trace/breakpoint trap.
	SIGTRAP,
	/// Process abort.
	SIGABRT,
	/// Access to an undefined portion of a memory object.
	SIGBUS,
	/// Erroneous arithmetic operation.
	SIGFPE,
	/// Kill.
	SIGKILL,
	/// User-defined signal 1.
	SIGUSR1,
	/// Invalid memory reference.
	SIGSEGV,
	/// User-defined signal 2.
	SIGUSR2,
	/// Write on a pipe with no one to read it.
	SIGPIPE,
	/// Alarm clock.
	SIGALRM,
	/// Termination.
	SIGTERM,
	/// Child process terminated.
	SIGCHLD,
	/// Continue executing.
	SIGCONT,
	/// Stop executing.
	SIGSTOP,
	/// Terminal stop.
	SIGTSTP,
	/// Background process attempting to read.
	SIGTTIN,
	/// Background process attempting to write.
	SIGTTOU,
	/// High bandwidth data is available at a socket.
	SIGURG,
	/// CPU time limit exceeded.
	SIGXCPU,
	/// File size limit exceeded.
	SIGXFSZ,
	/// Virtual timer expired.
	SIGVTALRM,
	/// Profiling timer expired.
	SIGPROF,
	/// Window resize.
	SIGWINCH,
	/// Pollable event.
	SIGPOLL,
	/// Bad system call.
	SIGSYS,
}

impl TryFrom<c_int> for Signal {
	type Error = Errno;

	/// `id` is the signal ID.
	fn try_from(id: c_int) -> Result<Self, Self::Error> {
		match id {
			1 => Ok(Self::SIGHUP),
			2 => Ok(Self::SIGINT),
			3 => Ok(Self::SIGQUIT),
			4 => Ok(Self::SIGILL),
			5 => Ok(Self::SIGTRAP),
			6 => Ok(Self::SIGABRT),
			7 => Ok(Self::SIGBUS),
			8 => Ok(Self::SIGFPE),
			9 => Ok(Self::SIGKILL),
			10 => Ok(Self::SIGUSR1),
			11 => Ok(Self::SIGSEGV),
			12 => Ok(Self::SIGUSR2),
			13 => Ok(Self::SIGPIPE),
			14 => Ok(Self::SIGALRM),
			15 => Ok(Self::SIGTERM),
			17 => Ok(Self::SIGCHLD),
			18 => Ok(Self::SIGCONT),
			19 => Ok(Self::SIGSTOP),
			20 => Ok(Self::SIGTSTP),
			21 => Ok(Self::SIGTTIN),
			22 => Ok(Self::SIGTTOU),
			23 => Ok(Self::SIGURG),
			24 => Ok(Self::SIGXCPU),
			25 => Ok(Self::SIGXFSZ),
			26 => Ok(Self::SIGVTALRM),
			27 => Ok(Self::SIGPROF),
			28 => Ok(Self::SIGWINCH),
			29 => Ok(Self::SIGPOLL),
			31 => Ok(Self::SIGSYS),
			_ => Err(errno!(EINVAL)),
		}
	}
}

impl Signal {
	/// Returns the signal's ID.
	pub const fn get_id(&self) -> u8 {
		match self {
			Self::SIGHUP => 1,
			Self::SIGINT => 2,
			Self::SIGQUIT => 3,
			Self::SIGILL => 4,
			Self::SIGTRAP => 5,
			Self::SIGABRT => 6,
			Self::SIGBUS => 7,
			Self::SIGFPE => 8,
			Self::SIGKILL => 9,
			Self::SIGUSR1 => 10,
			Self::SIGSEGV => 11,
			Self::SIGUSR2 => 12,
			Self::SIGPIPE => 13,
			Self::SIGALRM => 14,
			Self::SIGTERM => 15,
			Self::SIGCHLD => 17,
			Self::SIGCONT => 18,
			Self::SIGSTOP => 19,
			Self::SIGTSTP => 20,
			Self::SIGTTIN => 21,
			Self::SIGTTOU => 22,
			Self::SIGURG => 23,
			Self::SIGXCPU => 24,
			Self::SIGXFSZ => 25,
			Self::SIGVTALRM => 26,
			Self::SIGPROF => 27,
			Self::SIGWINCH => 28,
			Self::SIGPOLL => 29,
			Self::SIGSYS => 31,
		}
	}

	/// Returns the default action for the signal.
	pub fn get_default_action(&self) -> SignalAction {
		match self {
			Self::SIGHUP => SignalAction::Terminate,
			Self::SIGINT => SignalAction::Terminate,
			Self::SIGQUIT => SignalAction::Abort,
			Self::SIGILL => SignalAction::Abort,
			Self::SIGTRAP => SignalAction::Abort,
			Self::SIGABRT => SignalAction::Abort,
			Self::SIGBUS => SignalAction::Abort,
			Self::SIGFPE => SignalAction::Abort,
			Self::SIGKILL => SignalAction::Terminate,
			Self::SIGUSR1 => SignalAction::Terminate,
			Self::SIGSEGV => SignalAction::Abort,
			Self::SIGUSR2 => SignalAction::Terminate,
			Self::SIGPIPE => SignalAction::Terminate,
			Self::SIGALRM => SignalAction::Terminate,
			Self::SIGTERM => SignalAction::Terminate,
			Self::SIGCHLD => SignalAction::Ignore,
			Self::SIGCONT => SignalAction::Continue,
			Self::SIGSTOP => SignalAction::Stop,
			Self::SIGTSTP => SignalAction::Stop,
			Self::SIGTTIN => SignalAction::Stop,
			Self::SIGTTOU => SignalAction::Stop,
			Self::SIGURG => SignalAction::Ignore,
			Self::SIGXCPU => SignalAction::Abort,
			Self::SIGXFSZ => SignalAction::Abort,
			Self::SIGVTALRM => SignalAction::Terminate,
			Self::SIGPROF => SignalAction::Terminate,
			Self::SIGWINCH => SignalAction::Ignore,
			Self::SIGPOLL => SignalAction::Terminate,
			Self::SIGSYS => SignalAction::Abort,
		}
	}

	/// Tells whether the signal can be caught.
	pub fn can_catch(&self) -> bool {
		!matches!(
			self,
			Self::SIGKILL | Self::SIGSEGV | Self::SIGSTOP | Self::SIGSYS
		)
	}
}
