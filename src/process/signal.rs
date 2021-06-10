//! This module implements process signals.

use crate::errno::Errno;
use crate::errno;
use super::Process;
use super::State;

/// Type representing the type of a signal.
pub type SignalType = u8;

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
pub struct Signal {
	/// The signal type.
	type_: SignalType,

	// TODO
}

impl Signal {
	/// Creates a new instance.
	/// `type_` is the signal type.
	pub fn new(type_: SignalType) -> Result<Self, Errno> {
		if (type_ as usize) < SIGNALS_COUNT {
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

	/// Tells whether the signal can be caught.
	pub fn can_catch(&self) -> bool {
		self.type_ != SIGKILL && self.type_ != SIGSTOP
	}

	/// Executes the action associated with the signal for process `process`.
	pub fn execute_action(&self, process: &mut Process) {
		let process_state = process.get_state();
		if process_state == State::Zombie {
			return;
		}

		if let Some(_handler) = process.get_signal_handler(self.type_) {
			// TODO Execute handler
			todo!();
		} else {
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
		}
	}
}

/// Structure representing a signal handler.
#[derive(Clone, Copy)]
pub struct SignalHandler {
	// TODO
}
