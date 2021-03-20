/// The role of the process scheduler is to interrupt the currently running process periodicaly
/// to switch to another process that is in running state. The interruption is fired by the PIT
/// on IDT0.
///
/// A scheduler cycle is a period during which the scheduler iterates through every processes.
/// The scheduler works by assigning a number of quantum for each process, based on the number of
/// running processes and their priority.
/// This number represents the number of ticks during which the process keeps running until
/// switching to the next process.

use core::cmp::max;
use crate::event::InterruptCallback;
use crate::event;
use crate::gdt;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::process::tss;
use crate::util::Regs;
use crate::util::container::vec::Vec;
use crate::util::math;
use crate::util::ptr::SharedPtr;
use crate::util;

/// The number of quanta for the process with the average priority.
const AVERAGE_PRIORITY_QUANTA: usize = 10;
/// The number of quanta for the process with the maximum priority.
const MAX_PRIORITY_QUANTA: usize = 30;

extern "C" {
	fn context_switch(regs: &Regs, data_selector: u16, code_selector: u16) -> !;
	fn context_switch_kernel(regs: &Regs) -> !;
}

/// Scheduler ticking callback.
pub struct TickCallback {
	/// A reference to the scheduler.
	scheduler: SharedPtr<Scheduler>,
}

impl InterruptCallback for TickCallback {
	fn is_enabled(&self) -> bool {
		true
	}

	fn call(&mut self, _id: u32, _code: u32, regs: &util::Regs) {
		(*self.scheduler).tick(regs);
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

	/// The sum of all priorities, used to compute the average priority.
	priority_sum: usize,
	/// The priority of the processs which has the current highest priority.
	priority_max: usize,

	/// The current process cursor on the `processes` list.
	cursor: usize,
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub fn new() -> Result::<SharedPtr::<Self>, ()> {
		let mut s = SharedPtr::<Self>::new(Self {
			tick_callback: None,

			processes: Vec::<SharedPtr::<Process>>::new(),
			curr_proc: None,

			priority_sum: 0,
			priority_max: 0,

			cursor: 0,
		})?;
		(*s).tick_callback = Some(event::register_callback(32, 0, TickCallback {
			scheduler: s.clone(),
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

	/// Updates the scheduler's heuristic with the new priority of a process.
	/// `old` is the old priority of the process.
	/// `new` is the newe priority of the process.
	/// The function doesn't need to know the process which has been updated since it updates
	/// global informations.
	pub fn update_priority(&mut self, old: usize, new: usize) {
		self.priority_sum = self.priority_sum - old + new;
		if old >= self.priority_max {
			self.priority_max = new;
		}
	}

	/// Adds a process to the scheduler.
	pub fn add_process(&mut self, process: Process) -> Result::<SharedPtr::<Process>, ()> {
		let mut ptr = SharedPtr::new(process)?;
		self.processes.push(ptr.clone());
		self.update_priority(0, ptr.get_priority());

		Ok(ptr)
	}

	// TODO Remove process (don't forget to update the priority)

	/// Returns the average priority of a process.
	fn get_average_priority(&self) -> usize {
		self.priority_sum / self.processes.len()
	}

	/// Returns the number of quantum for the given priority.
	fn get_quantum_count(&self, priority: usize) -> usize {
		let n = math::integer_linear_interpolation::<isize>(priority as _,
			self.get_average_priority() as _,
			self.priority_max as _,
			AVERAGE_PRIORITY_QUANTA as _,
			MAX_PRIORITY_QUANTA as _);
		max(1, n) as _
	}

	/// Returns the next process to run.
	fn get_next_process(&mut self) -> Option::<&mut SharedPtr::<Process>> {
		if self.processes.is_empty() {
			return None;
		}

		if self.cursor >= self.processes.len() {
			self.cursor = 0;
		}

		let cursor_priority = self.processes[self.cursor].priority;
		let cursor_quantum_count = self.get_quantum_count(cursor_priority);
		if cursor_quantum_count >= cursor_quantum_count {
			let cursor_process = &mut self.processes[self.cursor];
			cursor_process.quantum_count = 0;
			self.cursor = (self.cursor + 1) % self.processes.len();
		}

		Some(&mut self.processes[self.cursor])
	}

	/// Ticking the scheduler. This function saves the data of the currently running process, then
	/// switches to the next process to run.
	fn tick(&mut self, regs: &util::Regs) {
		if let Some(curr_proc) = self.get_current_process() {
			curr_proc.regs = *regs;
		}

		if let Some(next_proc) = self.get_next_process() {
			self.curr_proc = Some(next_proc.clone());

			let curr_proc = self.curr_proc.as_mut().unwrap();
			curr_proc.quantum_count += 1;

			let tss = tss::get();
			tss.ss0 = gdt::KERNEL_DATA_OFFSET as _;
			tss.ss = gdt::USER_DATA_OFFSET as _;
			tss.esp0 = curr_proc.kernel_stack as _;
			curr_proc.mem_space.bind();

			let eip = curr_proc.regs.eip;
			let vmem = curr_proc.mem_space.get_vmem();
			debug_assert!(vmem.translate(eip as _).is_some());

			// TODO Handle syscalling
			unsafe { // Call to ASM function
				context_switch(&curr_proc.regs,
					(gdt::USER_DATA_OFFSET | 3) as _,
					(gdt::USER_CODE_OFFSET | 3) as _);
			}
		}
	}
}

impl Drop for Scheduler {
	fn drop(&mut self) {
		// TODO Unregister `tick_callback`
	}
}
