/// TODO doc

use crate::event::InterruptCallback;
use crate::event;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::util::container::vec::Vec;
use crate::util;

/// Scheduler ticking callback.
pub struct TickCallback {
	/// A reference to the scheduler.
	scheduler: *mut Scheduler,
}

impl InterruptCallback for TickCallback {
	fn is_enabled(&self) -> bool {
		true
	}

	fn call(&self, _id: u32, _code: u32, regs: &util::Regs) {
		unsafe { // Dereference of raw pointer
			(*self.scheduler).tick(regs);
		}
	}
}

/// The structure representing the process scheduler.
pub struct Scheduler {
	/// The list of all processes.
	processes: Vec::<Process>, // TODO Use another container to be able to take a reference of the content
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub fn new() -> Result::<Self, ()> {
		let mut s = Self {
			processes: Vec::<Process>::new(),
		};
		event::register_callback(32, 0, TickCallback {
			scheduler: &mut s as _,
		})?;

		Ok(s)
	}

	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns None.
	pub fn get_by_pid(&mut self, _pid: Pid) -> Option::<&'static mut Process> {
		// TODO
		None
	}

	///// Adds a process to the scheduler.
	//pub fn add_process(&mut self, process: Process) -> &'static mut Process {
	//	// TODO
	//}

	fn tick(&self, _regs: &util::Regs) {
		// TODO
		print!("Tick");
	}
}

impl Drop for Scheduler {
	fn drop(&mut self) {
		// TODO Unregister callback
	}
}
