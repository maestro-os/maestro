//! This module implements process signals.

mod signal_trampoline;

use core::ffi::c_void;
use core::mem::size_of;
use core::mem::transmute;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::Uid;
use crate::process::oom;
use crate::process::pid::Pid;
use crate::time::Clock;
use signal_trampoline::signal_trampoline;
use super::Process;
use super::State;

/// Type representing the type of a signal.
pub type SignalType = i32;
/// Type representing a signal handler.
pub type SigHandler = extern "C" fn(i32);

/// Process abort.
pub const SIGABRT: SignalType = 1;
/// Alarm clock.
pub const SIGALRM: SignalType = 2;
/// Access to an undefined portion of a memory object.
pub const SIGBUS: SignalType = 3;
/// Child process terminated.
pub const SIGCHLD: SignalType = 4;
/// Continue executing.
pub const SIGCONT: SignalType = 5;
/// Erroneous arithmetic operation.
pub const SIGFPE: SignalType = 6;
/// Hangup.
pub const SIGHUP: SignalType = 7;
/// Illigal instruction.
pub const SIGILL: SignalType = 8;
/// Terminal interrupt.
pub const SIGINT: SignalType = 9;
/// Kill.
pub const SIGKILL: SignalType = 10;
/// Write on a pipe with no one to read it.
pub const SIGPIPE: SignalType = 11;
/// Terminal quit.
pub const SIGQUIT: SignalType = 12;
/// Invalid memory reference.
pub const SIGSEGV: SignalType = 13;
/// Stop executing.
pub const SIGSTOP: SignalType = 14;
/// Termination.
pub const SIGTERM: SignalType = 15;
/// Terminal stop.
pub const SIGTSTP: SignalType = 16;
/// Background process attempting read.
pub const SIGTTIN: SignalType = 17;
/// Background process attempting write.
pub const SIGTTOU: SignalType = 18;
/// User-defined signal 1.
pub const SIGUSR1: SignalType = 19;
/// User-defined signal 2.
pub const SIGUSR2: SignalType = 20;
/// Pollable event.
pub const SIGPOLL: SignalType = 21;
/// Profiling timer expired.
pub const SIGPROF: SignalType = 22;
/// Bad system call.
pub const SIGSYS: SignalType = 23;
/// Trace/breakpoint trap.
pub const SIGTRAP: SignalType = 24;
/// High bandwidth data is available at a socket.
pub const SIGURG: SignalType = 25;
/// Virtual timer expired.
pub const SIGVTALRM: SignalType = 26;
/// CPU time limit exceeded.
pub const SIGXCPU: SignalType = 27;
/// File size limit exceeded.
pub const SIGXFSZ: SignalType = 28;
/// Window resize.
pub const SIGWINCH: SignalType = 29;

/// Ignoring the signal.
pub const SIG_IGN: *const c_void = 0x0 as _;
/// The default action for the signal.
pub const SIG_DFL: *const c_void = 0x1 as _;

/// The number of different signal types.
pub const SIGNALS_COUNT: usize = 29;

/// The size of the redzone in userspace, in bytes.
pub const REDZONE_SIZE: usize = 128;

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
union SigVal {
	/// The value as an int.
	sigval_int: i32,
	/// The value as a pointer.
	sigval_ptr: *mut c_void,
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
	si_utime: Clock,
	/// System time consumed.
	si_stime: Clock,
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
	/// A mask of signals that should be masked while executing the signal handler.
	pub sa_mask: SigSet,
	/// A set of flags which modifies the behaviour of the signal.
	pub sa_flags: i32,
	/// Unused.
	pub sa_restorer: Option<extern "C" fn()>,
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
				sa_handler: unsafe {
					transmute::<_, _>(SIG_IGN)
				},
				sa_sigaction: #[allow(invalid_value)] unsafe { core::mem::zeroed() },
				sa_mask: 0,
				sa_flags: 0,
				sa_restorer: #[allow(invalid_value)] unsafe { core::mem::zeroed() },
			},

			Self::Default => SigAction {
				sa_handler: unsafe {
					transmute::<_, _>(SIG_DFL)
				},
				sa_sigaction: #[allow(invalid_value)] unsafe { core::mem::zeroed() },
				sa_mask: 0,
				sa_flags: 0,
				sa_restorer: #[allow(invalid_value)] unsafe { core::mem::zeroed() },
			},

			Self::Handler(action) => action.clone(),
		}
	}
}

/// Array containing the default actions for each signal.
static DEFAULT_ACTIONS: &[SignalAction] = &[
	SignalAction::Ignore, // No signal
	SignalAction::Abort, // SIGABRT
	SignalAction::Terminate, // SIGALRM
	SignalAction::Abort, // SIGBUS
	SignalAction::Ignore, // SIGCHLD
	SignalAction::Continue, // SIGCONT
	SignalAction::Abort, // SIGFPE
	SignalAction::Terminate, // SIGHUP
	SignalAction::Abort, // SIGILL
	SignalAction::Terminate, // SIGINT
	SignalAction::Terminate, // SIGKILL
	SignalAction::Terminate, // SIGPIPE
	SignalAction::Abort, // SIGQUIT
	SignalAction::Abort, // SIGSEGV
	SignalAction::Stop, // SIGSTOP
	SignalAction::Terminate, // SIGTERM
	SignalAction::Stop, // SIGTSTP
	SignalAction::Stop, // SIGTTIN
	SignalAction::Stop, // SIGTTOU
	SignalAction::Terminate, // SIGUSR1
	SignalAction::Terminate, // SIGUSR2
	SignalAction::Terminate, // SIGPOLL
	SignalAction::Terminate, // SIGPROF
	SignalAction::Abort, // SIGSYS
	SignalAction::Abort, // SIGTRAP
	SignalAction::Ignore, // SIGURG
	SignalAction::Terminate, // SIGVTALRM
	SignalAction::Abort, // SIGXCPU
	SignalAction::Abort, // SIGXFSZ
	SignalAction::Ignore, // SIGWINCH
];

/// Structure representing a process signal.
#[derive(Clone)]
pub struct Signal {
	/// The signal type.
	type_: SignalType,

	// TODO
}

impl Signal {
	/// Creates a new instance.
	/// `type_` is the signal type.
	pub fn new(type_: SignalType) -> Result<Self, Errno> {
		if type_ >= 1 && (type_ - 1) < SIGNALS_COUNT as i32 {
			Ok(Self {
				type_,
			})
		} else {
			Err(errno!(EINVAL))
		}
	}

	/// Returns the signal's type.
	pub fn get_type(&self) -> SignalType {
		self.type_
	}

	/// Returns the default action for the signal.
	pub fn get_default_action(&self) -> SignalAction {
		DEFAULT_ACTIONS[self.type_ as usize]
	}

	/// Tells whether the signal can be caught.
	pub fn can_catch(&self) -> bool {
		self.type_ != SIGKILL && self.type_ != SIGSTOP && self.type_ != SIGSYS
	}

	/// Executes the action associated with the signal for process `process`.
	/// If the process is not the current process, the behaviour is undefined.
	/// If `no_handler` is true, the function executes the default action of the signal regardless
	/// the user-specified action.
	pub fn execute_action(&self, process: &mut Process, no_handler: bool) {
		process.signal_clear(self.type_);

		let process_state = process.get_state();
		if process_state == State::Zombie {
			return;
		}

		let handler = if !self.can_catch() || no_handler {
			SignalHandler::Default
		} else {
			process.get_signal_handler(self.type_)
		};

		if handler != SignalHandler::Ignore {
			let action = self.get_default_action();
			if action == SignalAction::Stop || action == SignalAction::Continue {
				process.set_waitable(self.type_ as _);
			}
		}

		match handler {
			SignalHandler::Ignore => {},
			SignalHandler::Default => {
				let default_action = DEFAULT_ACTIONS[self.type_ as usize];
				let exit_code = (128 + self.type_) as u32;

				match default_action {
					SignalAction::Terminate | SignalAction::Abort => {
						process.exit(exit_code);
					},

					SignalAction::Ignore => {},

					SignalAction::Stop => {
						// TODO Handle semaphores
						if process_state == State::Running {
							process.set_state(State::Stopped);
						}
					},

					SignalAction::Continue => {
						// TODO Handle semaphores
						if process_state == State::Stopped {
							process.set_state(State::Running);
						}
					},
				}
			},

			// TODO Handle sa_sigaction, sa_flags and sa_mask
			SignalHandler::Handler(action) => {
				if !process.is_handling_signal() {
					// TODO Handle the case where an alternate stack is specified (only if the
					// action has the flag)
					let mut regs = process.get_regs().clone();
					let redzone_end = regs.esp - REDZONE_SIZE as u32;

					let signal_data_size = size_of::<[u32; 2]>() as u32;
					let signal_esp = redzone_end - signal_data_size;

					// FIXME Don't write data out of the stack
					oom::wrap(|| {
						let mut mem_space_guard = process.get_mem_space().unwrap().lock();
						let mem_space = mem_space_guard.get_mut();

						debug_assert!(mem_space.is_bound());
						mem_space.alloc(signal_esp as *mut [u32; 2])
					});
					let signal_data = unsafe {
						slice::from_raw_parts_mut(signal_esp as *mut u32, 2)
					};

					// The pointer to the signal handler
					signal_data[1] = action.sa_handler.map(| f | f as _).unwrap_or(0);
					// The signal number
					signal_data[0] = self.type_ as _;

					let signal_trampoline = unsafe {
						transmute::<
							extern "C" fn(*const c_void, i32) -> !,
							*const c_void
						>(signal_trampoline)
					};

					// Setting the stack to point to the signal's data
					regs.esp = signal_esp;
					// Setting the program counter to point to the signal trampoline
					regs.eip = signal_trampoline as _;

					// Saves the current state of the process to be restored when the handler will
					// return
					process.signal_save(self.type_);
					// Setting the process's registers to call the signal handler
					process.set_regs(&regs);
				}
			},
		}
	}
}
