//! This module implements process state handling.
//! A module can have the following states:
//! - Running: The process is available for execution and might be resumed by the processes
//! scheduler
//! - Sleeping: The process is waiting for a resource to become available
//! - Stopped: The execution of the process is stopped until resumed by reception of a signal
//! - Zombie: The process has been halted and is waiting for its status to be retrieved
//!
//! While a process is sleeping, the scheduler shall poll for the event on which
//! the process is waiting. To do so, an instance of a structure implementing
//! `SleepPoll` is passed with the Sleeping state.

use super::Process;
use crate::util::boxed::Box;

/// Trait used to poll for events on which a process is waiting.
/// If polling succeeds, the process is woke up in order to continue execution.
pub trait SleepPoll {
	/// Polls for events. If the process msut wake up, the function returns
	/// `true`.
	fn poll(&self, proc: &mut Process) -> bool;
}

/// An enumeration containing possible states for a process.
pub enum State {
	/// The process is running or waiting to run.
	Running,
	/// The process is waiting for an event.
	Sleeping(Box<dyn SleepPoll>),
	/// The process has been stopped by a signal or by tracing.
	Stopped,
	/// The process has been killed.
	Zombie,
}

impl State {
	/// Returns the character associated with the state.
	pub fn get_char(&self) -> char {
		match self {
			Self::Running => 'R',
			Self::Sleeping(..) => 'S',
			Self::Stopped => 'T',
			Self::Zombie => 'Z',
		}
	}

	/// Returns the name of the state as string.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Running => "running",
			Self::Sleeping(..) => "sleeping",
			Self::Stopped => "stopped",
			Self::Zombie => "zombie",
		}
	}
}

/// A structure that represent a state where a process shouldn't be polled but
/// instead waked up by another process.
pub struct WakePoll {}

impl SleepPoll for WakePoll {
	fn poll(&self, _: &mut Process) -> bool {
		false
	}
}
