/// TODO doc

use crate::process::pid::Pid;
use crate::process::Process;
use crate::util::container::vec::Vec;

/// The structure representing the process scheduler.
pub struct Scheduler {
	/// The list of all processes.
	processes: Vec::<Process>, // TODO Use another container to be able to take a reference of the content
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub fn new() -> Self {
		Self {
			processes: Vec::<Process>::new(),
		}
	}

	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns None.
	pub fn get_by_pid(&mut self, _pid: Pid) -> Option::<&'static Process> {
		// TODO
		None
	}

	// TODO
}
