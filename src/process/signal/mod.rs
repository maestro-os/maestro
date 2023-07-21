//! This module implements process signals.

mod signal_trampoline;

use super::Process;
use super::State;
use crate::errno::Errno;
use crate::file::Uid;
use crate::process::oom;
use crate::process::pid::Pid;
use crate::time::unit::ClockIdT;
use core::ffi::c_int;
use core::ffi::c_void;
use core::fmt;
use core::fmt::Debug;
use core::mem::size_of;
use core::mem::transmute;
use core::slice;
use signal_trampoline::signal_trampoline;

/// Type representing a signal handler.
pub type SigHandler = extern "C" fn(i32);

/// Ignoring the signal.
pub const SIG_IGN: *const c_void = 0x0 as _;
/// The default action for the signal.
pub const SIG_DFL: *const c_void = 0x1 as _;

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

/// Union representing a signal value.
#[repr(C)]
#[derive(Clone, Copy)]
pub union SigVal {
	/// The value as an int.
	pub sigval_int: i32,
	/// The value as a pointer.
	pub sigval_ptr: *mut c_void,
}

impl Debug for SigVal {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let val = unsafe { self.sigval_ptr };
		write!(fmt, "SigVal {{ sigval_ptr: {:p} }}", val)
	}
}

/// Structure storing signal informations.
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

// TODO Check the type is correct
/// Type representing a signal mask.
pub type SigSet = u32;

/// Structure storing an action to be executed when a signal is received.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SigAction {
	/// The action associated with the signal.
	pub sa_handler: Option<SigHandler>,
	/// Used instead of `sa_handler` if SA_SIGINFO is specified in `sa_flags`.
	pub sa_sigaction: Option<extern "C" fn(i32, *mut SigInfo, *mut c_void)>,
	/// A mask of signals that should be masked while executing the signal
	/// handler.
	pub sa_mask: SigSet,
	/// A set of flags which modifies the behaviour of the signal.
	pub sa_flags: i32,
	/// Unused.
	pub sa_restorer: Option<extern "C" fn()>,
}

/// Structure for notification from asynchronous routines.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct SigEvent {
	/// Notification method.
	pub sigev_notify: c_int,
	/// Notification signal.
	pub sigev_signo: c_int,
	/// TODO doc
	pub sigev_value: SigVal,
	/// Data passed with notification.
	pub sigev_notify_function: extern "C" fn(SigVal),
	/// Fucnction used for thread notification.
	pub sigev_notify_attributes: *const c_void,
	/// ID of thread to signal.
	pub sigev_notify_thread_id: Pid,
}

impl SigEvent {
	/// Tells whether the structure is valid.
	pub fn is_valid(&self) -> bool {
		if !matches!(self.sigev_notify, SIGEV_SIGNAL | SIGEV_NONE | SIGEV_THREAD) {
			return false;
		}
		if Signal::try_from(self.sigev_signo as u32).is_err() {
			return false;
		}
		// TODO check sigev_notify_thread_id

		true
	}
}

/// Enumeration containing the different possibilities for signal handling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalHandler {
	/// Ignores the signal.
	Ignore,
	/// Executes the default action.
	Default,
	/// A custom action defined with a call to signal.
	Handler(SigAction),
}

impl SignalHandler {
	/// Returns an instance of SigAction associated with the handler.
	pub fn get_action(&self) -> SigAction {
		match self {
			Self::Ignore => SigAction {
				sa_handler: unsafe { transmute::<_, _>(SIG_IGN) },
				sa_sigaction: #[allow(invalid_value)]
				unsafe {
					core::mem::zeroed()
				},
				sa_mask: 0,
				sa_flags: 0,
				sa_restorer: #[allow(invalid_value)]
				unsafe {
					core::mem::zeroed()
				},
			},

			Self::Default => SigAction {
				sa_handler: unsafe { transmute::<_, _>(SIG_DFL) },
				sa_sigaction: #[allow(invalid_value)]
				unsafe {
					core::mem::zeroed()
				},
				sa_mask: 0,
				sa_flags: 0,
				sa_restorer: #[allow(invalid_value)]
				unsafe {
					core::mem::zeroed()
				},
			},

			Self::Handler(action) => *action,
		}
	}
}

/// Enumeration of signal types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Signal {
	/// Hangup.
	SIGHUP,
	/// Terminal interrupt.
	SIGINT,
	/// Terminal quit.
	SIGQUIT,
	/// Illigal instruction.
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
	/// Background process attempting read.
	SIGTTIN,
	/// Background process attempting write.
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

impl TryFrom<u32> for Signal {
	type Error = Errno;

	/// `id` is the signal ID.
	fn try_from(id: u32) -> Result<Self, Self::Error> {
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

	/// Executes the action associated with the signal for process `process`.
	///
	/// If the process is not the current process, the behaviour is undefined.
	///
	/// If `no_handler` is `true`, the function executes the default action of the
	/// signal regardless the user-specified action.
	pub fn execute_action(&self, process: &mut Process, no_handler: bool) {
		process.signal_clear(self.clone());

		let process_state = process.get_state();
		if matches!(process_state, State::Zombie) {
			return;
		}

		let handler = if !self.can_catch() || no_handler {
			SignalHandler::Default
		} else {
			process.get_signal_handler(self)
		};

		match handler {
			SignalHandler::Ignore => {}
			SignalHandler::Default => {
				// Signals on the init process can be executed only if the process has set a
				// signal handler
				if self.can_catch() && process.is_init() {
					return;
				}

				let action = self.get_default_action();
				match action {
					SignalAction::Terminate | SignalAction::Abort => {
						process.exit(self.get_id() as _, true);
					}

					SignalAction::Ignore => {}

					SignalAction::Stop => {
						// TODO Handle semaphores
						if matches!(process_state, State::Running) {
							process.set_state(State::Stopped);
						}

						process.set_waitable(self.get_id());
					}

					SignalAction::Continue => {
						// TODO Handle semaphores
						if matches!(process_state, State::Stopped) {
							process.set_state(State::Running);
						}

						process.set_waitable(self.get_id());
					}
				}
			}

			// TODO Handle sa_sigaction, sa_flags and sa_mask
			SignalHandler::Handler(action) if !process.is_handling_signal() => {
				// TODO Handle the case where an alternate stack is specified (only if the
				// action has the flag)
				// The signal handler stack
				let stack = process.get_signal_stack();

				let signal_data_size = size_of::<[u32; 3]>();
				let signal_esp = (stack as usize) - signal_data_size;

				// FIXME Don't write data out of the stack
				oom::wrap(|| {
					let mem_space = process.get_mem_space().unwrap();
					let mut mem_space = mem_space.lock();

					mem_space.bind();
					mem_space.alloc(signal_esp as *mut u32, 3)
				});
				let signal_data = unsafe { slice::from_raw_parts_mut(signal_esp as *mut u32, 3) };

				// The signal number
				signal_data[2] = self.get_id() as _;
				// The pointer to the signal handler
				signal_data[1] = action.sa_handler.map(|f| f as usize).unwrap_or(0) as _;
				// Padding (return pointer)
				signal_data[0] = 0;

				let signal_trampoline = unsafe {
					transmute::<extern "C" fn(*const c_void, i32) -> !, *const c_void>(
						signal_trampoline,
					)
				};

				let mut regs = process.regs.clone();
				// Setting the stack to point to the signal's data
				regs.esp = signal_esp as _;
				// Setting the program counter to point to the signal trampoline
				regs.eip = signal_trampoline as _;

				// Saves the current state of the process to be restored when the handler will
				// return
				process.signal_save(self.clone());
				// Setting the process's registers to call the signal handler
				process.regs = regs;
			}

			_ => {}
		}
	}
}
