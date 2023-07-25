//! The role of the process scheduler is to interrupt the currently running
//! process periodicaly to switch to another process that is in running state.
//!
//! The interruption is fired by the PIT on IDT0.
//!
//! A scheduler cycle is a period during which the scheduler iterates through
//! every processes. The scheduler works by assigning a number of quantum for
//! each process, based on the number of running processes and their priority.
//! This number represents the number of ticks during which the process keeps
//! running until switching to the next process.

use crate::errno::Errno;
use crate::event;
use crate::event::CallbackHook;
use crate::idt::pic;
use crate::memory;
use crate::memory::malloc;
use crate::memory::stack;
use crate::process;
use crate::process::pid::Pid;
use crate::process::regs::Regs;
use crate::process::Process;
use crate::process::State;
use crate::time;
use crate::util::container::map::Map;
use crate::util::container::map::MapIterator;
use crate::util::container::vec::Vec;
use crate::util::lock::*;
use crate::util::math;
use crate::util::math::rational::Rational;
use crate::util::ptr::arc::Arc;
use core::arch::asm;
use core::cmp::max;
use core::ffi::c_void;

/// The size of the temporary stack for context switching.
const TMP_STACK_SIZE: usize = 16 * memory::PAGE_SIZE;
/// The number of quanta for the process with the average priority.
const AVERAGE_PRIORITY_QUANTA: usize = 10;
/// The number of quanta for the process with the maximum priority.
const MAX_PRIORITY_QUANTA: usize = 30;

/// The structure representing the process scheduler.
pub struct Scheduler {
	/// A vector containing the temporary stacks for each CPU cores.
	tmp_stacks: Vec<malloc::Alloc<u8>>,

	/// The ticking callback hook, called at a regular interval to make the
	/// scheduler work.
	tick_callback_hook: CallbackHook,
	/// The total number of ticks since the instanciation of the scheduler.
	total_ticks: u64,

	/// A binary tree containing all processes registered to the current
	/// scheduler.
	processes: Map<Pid, Arc<IntMutex<Process>>>,
	/// The currently running process with its PID.
	curr_proc: Option<(Pid, Arc<IntMutex<Process>>)>,

	/// The current number of running processes.
	running_procs: usize,

	/// The sum of all priorities, used to compute the average priority.
	priority_sum: usize,
	/// The priority of the processs which has the current highest priority.
	priority_max: usize,
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub fn new(cores_count: usize) -> Result<Arc<IntMutex<Self>>, Errno> {
		let mut tmp_stacks = Vec::new();
		for _ in 0..cores_count {
			tmp_stacks.push(malloc::Alloc::new_default(TMP_STACK_SIZE)?)?;
		}

		// Register tick handler
		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();
		let tick_callback_hook = event::register_callback(
			pit.get_interrupt_vector(),
			|_: u32, _: u32, regs: &Regs, ring: u32| {
				Scheduler::tick(process::get_scheduler(), regs, ring);
			},
		)?;

		Arc::new(IntMutex::new(Self {
			tmp_stacks,

			tick_callback_hook,
			total_ticks: 0,

			processes: Map::new(),
			curr_proc: None,

			running_procs: 0,

			priority_sum: 0,
			priority_max: 0,
		}))
	}

	/// Returns a pointer to the top of the tmp stack for the given core `core`.
	pub fn get_tmp_stack(&mut self, core: u32) -> *mut c_void {
		unsafe {
			self.tmp_stacks[core as usize]
				.as_ptr_mut()
				.add(TMP_STACK_SIZE) as *mut _
		}
	}

	/// Returns the total number of ticks since the instanciation of the
	/// scheduler.
	pub fn get_total_ticks(&self) -> u64 {
		self.total_ticks
	}

	/// Returns an iterator on the scheduler's processes.
	pub fn iter_process(&mut self) -> MapIterator<'_, Pid, Arc<IntMutex<Process>>> {
		self.processes.iter()
	}

	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(&self, pid: Pid) -> Option<Arc<IntMutex<Process>>> {
		Some(self.processes.get(pid)?.clone())
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(&self, _tid: Pid) -> Option<Arc<IntMutex<Process>>> {
		// TODO
		todo!();
	}

	/// Returns the current running process.
	///
	/// If no process is running, the function returns `None`.
	pub fn get_current_process(&mut self) -> Option<Arc<IntMutex<Process>>> {
		Some(self.curr_proc.as_ref().cloned()?.1)
	}

	/// Updates the scheduler's heuristic with the new priority of a process.
	///
	/// Arguments:
	/// - `old` is the old priority of the process.
	/// - `new` is the new priority of the process.
	///
	/// The function doesn't need to know the process which has been updated
	/// since it updates global informations.
	pub fn update_priority(&mut self, old: usize, new: usize) {
		self.priority_sum = self.priority_sum - old + new;

		if new >= self.priority_max {
			self.priority_max = new;
		}

		// FIXME: Unable to determine priority_max when new < old
	}

	/// Adds a process to the scheduler.
	pub fn add_process(&mut self, process: Process) -> Result<Arc<IntMutex<Process>>, Errno> {
		let pid = process.pid;
		let priority = process.priority;

		if *process.get_state() == State::Running {
			self.increment_running();
		}

		let ptr = Arc::new(IntMutex::new(process))?;
		self.processes.insert(pid, ptr.clone())?;
		self.update_priority(0, priority);

		Ok(ptr)
	}

	/// Removes the process with the given pid `pid`.
	pub fn remove_process(&mut self, pid: Pid) {
		if let Some(proc_mutex) = self.get_by_pid(pid) {
			let proc = proc_mutex.lock();

			if *proc.get_state() == State::Running {
				self.decrement_running();
			}

			let priority = proc.priority;
			self.processes.remove(&pid);
			self.update_priority(priority, 0);
		}
	}

	/// Returns the current ticking frequency of the scheduler.
	pub fn get_ticking_frequency(&self) -> Rational {
		Rational::from_integer((10 * self.running_procs) as _)
	}

	/// Increments the number of running processes.
	pub fn increment_running(&mut self) {
		self.running_procs += 1;

		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();

		if self.running_procs > 1 {
			pit.set_frequency(self.get_ticking_frequency());
			pit.set_enabled(true);
		}
	}

	/// Decrements the number of running processes.
	pub fn decrement_running(&mut self) {
		self.running_procs -= 1;

		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();

		if self.running_procs <= 1 {
			pit.set_enabled(false);
		} else {
			pit.set_frequency(self.get_ticking_frequency());
		}
	}

	// TODO Clean
	/// Returns the number of quantum for the given priority.
	///
	/// Arguments:
	/// - `priority` is the process's priority.
	/// - `priority_sum` is the sum of all processes' priorities.
	/// - `priority_max` is the highest priority a process currently has.
	/// - `processes_count` is the number of processes.
	fn get_quantum_count(
		priority: usize,
		priority_sum: usize,
		priority_max: usize,
		processes_count: usize,
	) -> usize {
		let n = math::integer_linear_interpolation::<isize>(
			priority as _,
			(priority_sum / processes_count) as _,
			priority_max as _,
			AVERAGE_PRIORITY_QUANTA as _,
			MAX_PRIORITY_QUANTA as _,
		);
		max(1, n) as _
	}

	// TODO Clean
	/// Tells whether the given process `process` can run.
	///
	/// TODO args
	fn can_run(
		process: &Process,
		_priority_sum: usize,
		_priority_max: usize,
		_processes_count: usize,
	) -> bool {
		if process.can_run() {
			// TODO fix
			//process.quantum_count < Self::get_quantum_count(process.get_priority(),
			// priority_sum, 	priority_max, processes_count)
			true
		} else {
			false
		}
	}

	// TODO Clean
	/// Returns the next process to run with its PID.
	///
	/// If the process is changed, the quantum count of the previous process is reset.
	fn get_next_process(&self) -> Option<(Pid, Arc<IntMutex<Process>>)> {
		let priority_sum = self.priority_sum;
		let priority_max = self.priority_max;
		let processes_count = self.processes.len();

		// Getting the current process, or take the first process in the list if no
		// process is running
		let (curr_pid, curr_proc) = self.curr_proc.clone().or_else(|| {
			self.processes
				.iter()
				.next()
				.map(|(pid, proc)| (*pid, proc.clone()))
		})?;

		let process_filter = |(_, proc): &(&Pid, &Arc<IntMutex<Process>>)| {
			let guard = proc.lock();
			Self::can_run(&guard, priority_sum, priority_max, processes_count)
		};

		let next_proc = self
			.processes
			.range((curr_pid + 1)..)
			.find(process_filter)
			.or_else(|| {
				// If no suitable process is found, go back to the beginning to check processes
				// located before the previous process (looping)

				self.processes.iter().find(process_filter)
			})
			.map(|(pid, proc)| (*pid, proc));

		let (next_pid, next_proc) = next_proc?;
		if next_pid != curr_pid || processes_count == 1 {
			curr_proc.lock().quantum_count = 0;
		}
		Some((next_pid, next_proc.clone()))
	}

	/// Ticking the scheduler.
	///
	/// This function saves the data of the currently running process, then switches to the next
	/// process to run.
	///
	/// If no process is ready to run, the scheduler halts the system until a process is runnable.
	///
	/// Arguments:
	/// - `sched_mutex` is the scheduler's mutex.
	/// - `regs` is the state of the registers from the paused context.
	/// - `ring` is the ring of the paused context.
	fn tick(sched_mutex: &IntMutex<Self>, regs: &Regs, ring: u32) -> ! {
		// Disabling interrupts to avoid getting one right after unlocking mutexes
		cli!();

		let tmp_stack = {
			let mut sched = sched_mutex.lock();
			sched.total_ticks += 1;

			// If a process is running, save its registers
			if let Some(curr_proc) = sched.get_current_process() {
				let mut curr_proc = curr_proc.lock();

				curr_proc.regs = regs.clone();
				curr_proc.syscalling = ring < 3;
			}

			// The current core ID
			let core_id = 0; // TODO
			sched.get_tmp_stack(core_id)
		};

		loop {
			let mut sched = sched_mutex.lock();

			if let Some(next_proc) = sched.get_next_process() {
				// Set the process as current
				sched.curr_proc = Some(next_proc.clone());

				drop(sched);

				unsafe {
					stack::switch(Some(tmp_stack), move || {
						let (resume, syscalling, regs) = {
							let mut next_proc = next_proc.1.lock();

							next_proc.prepare_switch();

							let resume = matches!(next_proc.get_state(), State::Running);
							(resume, next_proc.syscalling, next_proc.regs.clone())
						};
						drop(next_proc);

						if !resume {
							return;
						}

						// Resuming execution
						event::unlock_callbacks(0x20);
						pic::end_of_interrupt(0x0);

						regs.switch(!syscalling);
					})
					.unwrap();
				}
			} else {
				// No process to run. Just wait
				break;
			}
		}

		{
			sched_mutex.lock().curr_proc = None;
		}

		unsafe {
			event::unlock_callbacks(0x20);
			pic::end_of_interrupt(0x0);
			crate::loop_reset(tmp_stack);
		}
	}
}

/// Ends the current tick on the current CPU.
///
/// Since this function triggers an interruption, the caller must ensure that no criticl mutex is
/// locked, that could be used in the inerruption handler. Otherwise, a deadlock could occure.
#[inline]
pub fn end_tick() {
	unsafe {
		asm!("int 0x20");
	}
}
