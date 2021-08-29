//! This module implements process signals.

mod signal_trampoline;

use core::ffi::c_void;
use core::mem::size_of;
use core::mem::transmute;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use signal_trampoline::signal_trampoline;
use super::Process;
use super::State;

/// Type representing the type of a signal.
pub type SignalType = i32;
/// Type representing a signal handler.
pub type SigHandler = extern "C" fn(i32);

/// Process abort.
pub const SIGABRT: SignalType = 0;
/// Alarm clock.
pub const SIGALRM: SignalType = 1;
/// Access to an undefined portion of a memory object.
pub const SIGBUS: SignalType = 2;
/// Child process terminated.
pub const SIGCHLD: SignalType = 3;
/// Continue executing.
pub const SIGCONT: SignalType = 4;
/// Erroneous arithmetic operation.
pub const SIGFPE: SignalType = 5;
/// Hangup.
pub const SIGHUP: SignalType = 6;
/// Illigal instruction.
pub const SIGILL: SignalType = 7;
/// Terminal interrupt.
pub const SIGINT: SignalType = 8;
/// Kill.
pub const SIGKILL: SignalType = 9;
/// Write on a pipe with no one to read it.
pub const SIGPIPE: SignalType = 10;
/// Terminal quit.
pub const SIGQUIT: SignalType = 11;
/// Invalid memory reference.
pub const SIGSEGV: SignalType = 12;
/// Stop executing.
pub const SIGSTOP: SignalType = 13;
/// Termination.
pub const SIGTERM: SignalType = 14;
/// Terminal stop.
pub const SIGTSTP: SignalType = 15;
/// Background process attempting read.
pub const SIGTTIN: SignalType = 16;
/// Background process attempting write.
pub const SIGTTOU: SignalType = 17;
/// User-defined signal 1.
pub const SIGUSR1: SignalType = 18;
/// User-defined signal 2.
pub const SIGUSR2: SignalType = 19;
/// Pollable event.
pub const SIGPOLL: SignalType = 20;
/// Profiling timer expired.
pub const SIGPROF: SignalType = 21;
/// Bad system call.
pub const SIGSYS: SignalType = 22;
/// Trace/breakpoint trap.
pub const SIGTRAP: SignalType = 23;
/// High bandwidth data is available at a socket.
pub const SIGURG: SignalType = 24;
/// Virtual timer expired.
pub const SIGVTALRM: SignalType = 25;
/// CPU time limit exceeded.
pub const SIGXCPU: SignalType = 26;
/// File size limit exceeded.
pub const SIGXFSZ: SignalType = 27;
/// Window resize.
pub const SIGWINCH: SignalType = 28;

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

/// Enumeration containing the different possibilities for signal handling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalHandler {
	/// Ignores the signal.
	Ignore,
	/// Executes the default action.
	Default,
	/// A custom action defined by the process.
	Handler(SigHandler),
}

/// Array containing the default actions for each signal.
static DEFAULT_ACTIONS: &[SignalAction] = &[
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
		if type_ >= 0 && type_ < SIGNALS_COUNT as i32 {
			Ok(Self {
				type_,
			})
		} else {
			Err(errno::EINVAL)
		}
	}

	/// Returns the signal's type.
	pub fn get_type(&self) -> SignalType {
		self.type_
	}

	/// Tells whether the signal is a continuation signal.
	pub fn is_continuation(&self) -> bool {
		DEFAULT_ACTIONS[self.type_ as usize] == SignalAction::Continue
	}

	/// Tells whether the signal can be caught.
	pub fn can_catch(&self) -> bool {
		self.type_ != SIGKILL && self.type_ != SIGSTOP && self.type_ != SIGSYS
	}

	/// Executes the action associated with the signal for process `process`.
	/// If the process is not the current process, the behaviour is undefined.
	pub fn execute_action(&self, process: &mut Process) {
		debug_assert!(process.get_mem_space().unwrap().is_bound());

		let process_state = process.get_state();
		if process_state == State::Zombie {
			return;
		}

		let handler = if self.can_catch() {
			process.get_signal_handler(self.type_)
		} else {
			SignalHandler::Default
		};

		if handler != SignalHandler::Ignore {
			process.set_waitable(self.type_ as _);
		}

		match handler {
			SignalHandler::Ignore => {},
			SignalHandler::Default => {
				let default_action = DEFAULT_ACTIONS[self.type_ as usize];
				let exit_code = (128 + self.type_) as u32;

				match default_action {
					SignalAction::Terminate => {
						process.exit(exit_code);
					},

					SignalAction::Abort => {
						process.exit(exit_code);
					},

					SignalAction::Ignore => {},

					SignalAction::Stop => {
						// TODO Handle semaphores
						if process_state == State::Running {
							process.set_state(State::Sleeping);
						}
					},

					SignalAction::Continue => {
						// TODO Handle semaphores
						if process_state == State::Sleeping {
							process.set_state(State::Running);
						}
					},
				}
			},

			SignalHandler::Handler(handler) => {
				let mut regs = process.get_regs().clone();
				let redzone_end = regs.esp - REDZONE_SIZE as u32;

				let signal_data_size = size_of::<[u32; 2]>() as u32;
				let signal_esp = redzone_end - signal_data_size;

				// TODO Don't write data out of the stack
				process.get_mem_space_mut().unwrap().alloc(signal_esp as *mut [u32; 2]).unwrap();
				let signal_data = unsafe {
					slice::from_raw_parts_mut(signal_esp as *mut u32, 2)
				};

				// The pointer to the signal handler
				signal_data[1] = handler as _;
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
			},
		}
	}
}
