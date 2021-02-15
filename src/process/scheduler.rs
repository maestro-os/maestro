/// The role of the process scheduler is to interrupt the currently running process periodicaly
/// to switch to another process that is in running state. The interruption is fired by the PIT
/// on IDT0.

use crate::event::InterruptCallback;
use crate::event;
use crate::process::Process;
use crate::process::State;
use crate::process::pid::Pid;
use crate::util::Regs;
use crate::util::container::vec::Vec;
use crate::util::ptr::SharedPtr;
use crate::util;

extern "C" {
	fn context_switch(regs: &Regs, data_selector: u16, code_selector: u16) -> !;
	fn context_switch_kernel(regs: &Regs) -> !;
}

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
	/// The ticking callback, called at a regular interval to make the scheduler work.
	tick_callback: Option::<SharedPtr::<TickCallback>>,

	/// The list of all processes.
	processes: Vec::<SharedPtr::<Process>>,
	/// The currently running process.
	curr_proc: Option::<SharedPtr::<Process>>,
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub fn new() -> Result::<Self, ()> {
		let mut s = Self {
			tick_callback: None,

			processes: Vec::<SharedPtr::<Process>>::new(),
			curr_proc: None,
		};
		s.tick_callback = Some(event::register_callback(32, 0, TickCallback {
			scheduler: &mut s as _,
		})?);
		Ok(s)
	}

	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns None.
	pub fn get_by_pid(&mut self, _pid: Pid) -> Option::<SharedPtr::<Process>> {
		// TODO
		None
	}

	/// Returns the current running process. If no process is running, the function returns None.
	pub fn get_current_process(&mut self) -> Option::<&mut SharedPtr::<Process>> {
		self.curr_proc.as_mut()
	}

	/// Adds a process to the scheduler.
	pub fn add_process(&mut self, process: Process) -> Result::<SharedPtr::<Process>, ()> {
		let mut ptr = SharedPtr::new(process)?;
		self.processes.push(ptr.clone());

		if self.curr_proc.is_none() && ptr.get_current_state() == State::Running {
			self.curr_proc = Some(ptr.clone());
		}

		Ok(ptr)
	}

	/// Returns the next process to run.
	fn get_next_process(&mut self) -> Option::<&mut SharedPtr::<Process>> {
		// TODO
		self.curr_proc.as_mut()
	}

	/// Ticking the scheduler. This function saves the data of the currently running process, then
	/// switches to the next process to run.
	fn tick(&mut self, regs: &util::Regs) {
		unsafe { println!("Tick {}", regs.eip); }; // TODO rm
		if let Some(curr_proc) = self.get_current_process() {
			println!("Update {:p}", curr_proc);
			curr_proc.regs = *regs;
		}

		if let Some(next_proc) = self.get_next_process() {
			next_proc.mem_space.bind();

			unsafe { // Call to ASM function
				println!("Switching {:p} {}", next_proc, next_proc.regs.eip); // TODO rm
				context_switch(&next_proc.regs, 32 | 3, 24 | 3); // TODO Clean
			}
		}
	}
}

impl Drop for Scheduler {
	fn drop(&mut self) {
		// TODO Unregister `tick_callback`
	}
}
