/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The role of the process scheduler is to interrupt the currently running
//! process periodically to switch to another process that is in running state.
//!
//! A scheduler cycle is a period during which the scheduler iterates through
//! each process. The scheduler works by assigning a number of quantum for
//! each process, based on the number of running processes and their priority.
//! This number represents the number of ticks during which the process keeps
//! running until switching to the next process.

use crate::{
	event,
	event::CallbackHook,
	idt::pic,
	memory,
	memory::stack,
	process::{pid::Pid, regs::Regs, Process, State},
	time,
};
use core::arch::asm;
use utils::{
	collections::{
		btreemap::{BTreeMap, MapIterator},
		vec::Vec,
	},
	errno::AllocResult,
	interrupt::cli,
	lock::{once::OnceInit, IntMutex},
	math::rational::Rational,
	ptr::arc::Arc,
	vec,
};

// TODO handle processes priority

/// The size of the temporary stack for context switching.
const TMP_STACK_SIZE: usize = 16 * memory::PAGE_SIZE;

/// The processes scheduler.
pub static SCHEDULER: OnceInit<IntMutex<Scheduler>> = unsafe { OnceInit::new() };

/// Initializes schedulers.
pub fn init() -> AllocResult<()> {
	// TODO handle multicore
	unsafe {
		SCHEDULER.init(IntMutex::new(Scheduler::new()?));
	}
	Ok(())
}

/// A process scheduler.
///
/// Each CPU core has its own scheduler.
pub struct Scheduler {
	/// The ticking callback hook, called at a regular interval to make the
	/// scheduler work.
	tick_callback_hook: CallbackHook,
	/// The total number of ticks since the instantiation of the scheduler.
	total_ticks: u64,
	/// The scheduler's temporary stacks.
	tmp_stack: Vec<u8>,

	/// A binary tree containing all processes registered to the current
	/// scheduler.
	processes: BTreeMap<Pid, Arc<IntMutex<Process>>>,
	/// The process currently being executed by the scheduler's core, along with its PID.
	curr_proc: Option<(Pid, Arc<IntMutex<Process>>)>,
	/// The current number of processes in running state.
	running_procs: usize,
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub(super) fn new() -> AllocResult<Self> {
		// Allocate context switching stacks
		let tmp_stack = vec![0; TMP_STACK_SIZE]?;
		// Register tick callback
		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();
		let tick_callback_hook = event::register_callback(
			pit.get_interrupt_vector(),
			|_: u32, _: u32, regs: &Regs, ring: u32| {
				Scheduler::tick(SCHEDULER.get(), regs, ring);
			},
		)?
		.unwrap();
		Ok(Self {
			tick_callback_hook,
			total_ticks: 0,
			tmp_stack,

			processes: BTreeMap::new(),
			curr_proc: None,
			running_procs: 0,
		})
	}

	/// Returns a pointer to the top of the tmp stack for the given kernel `kernel`.
	pub fn get_tmp_stack(&mut self) -> *mut u8 {
		unsafe { self.tmp_stack.as_mut_ptr().add(self.tmp_stack.len()) }
	}

	/// Returns the total number of ticks since the instanciation of the
	/// scheduler.
	pub fn get_total_ticks(&self) -> u64 {
		self.total_ticks
	}

	/// Returns an iterator on the scheduler's processes.
	pub fn iter_process(&self) -> MapIterator<'_, Pid, Arc<IntMutex<Process>>> {
		self.processes.iter()
	}

	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(&self, pid: Pid) -> Option<Arc<IntMutex<Process>>> {
		Some(self.processes.get(&pid)?.clone())
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
	/// since it updates global information.
	fn update_priority(&mut self, _old: usize, _new: usize) {
		// TODO
	}

	/// Adds a process to the scheduler.
	pub fn add_process(&mut self, process: Process) -> AllocResult<Arc<IntMutex<Process>>> {
		if process.get_state() == State::Running {
			self.increment_running();
		}
		let pid = process.pid.get();
		let priority = process.priority;
		let ptr = Arc::new(IntMutex::new(process))?;
		self.processes.insert(pid, ptr.clone())?;
		self.update_priority(0, priority);
		Ok(ptr)
	}

	/// Removes the process with the given pid `pid`.
	pub fn remove_process(&mut self, pid: Pid) {
		let Some(proc_mutex) = self.get_by_pid(pid) else {
			return;
		};
		let proc = proc_mutex.lock();
		if proc.get_state() == State::Running {
			self.decrement_running();
		}
		self.processes.remove(&pid);
		self.update_priority(proc.priority, 0);
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

	/// Returns the next process to run with its PID.
	fn get_next_process(&self) -> Option<(Pid, Arc<IntMutex<Process>>)> {
		// Get the current process, or take the first process in the list if no
		// process is running
		let curr_pid = self
			.curr_proc
			.as_ref()
			.map(|(pid, _)| *pid)
			.or_else(|| self.processes.first_key_value().map(|(pid, _)| *pid))?;
		let process_filter = |(_, proc_mutex): &(&Pid, &Arc<IntMutex<Process>>)| {
			let proc = proc_mutex.lock();
			proc.can_run()
		};
		self.processes
			.range((curr_pid + 1)..)
			.find(process_filter)
			.or_else(|| {
				// If no suitable process is found, go back to the beginning to check processes
				// located before the previous process (looping)
				self.processes.range(..=curr_pid).find(process_filter)
			})
			.map(|(pid, proc)| (*pid, proc.clone()))
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
		// Disable interrupts so that they remain disabled between the time the scheduler is
		// unlocked and the context is switched to the next process
		cli();
		// Use a scope to drop mutex guards
		let (switch_info, tmp_stack) = {
			let mut sched = sched_mutex.lock();
			sched.total_ticks = sched.total_ticks.saturating_add(1);
			// If a process is running, save its registers
			if let Some(curr_proc) = sched.get_current_process() {
				let mut curr_proc = curr_proc.lock();
				curr_proc.regs = regs.clone();
				curr_proc.syscalling = ring < 3;
			}
			// Loop until a runnable process is found
			let (proc, switch_info) = loop {
				let Some((pid, proc_mutex)) = sched.get_next_process() else {
					// No process to run
					break (None, None);
				};
				// Try switching
				let mut proc = proc_mutex.lock();
				proc.prepare_switch();
				// If the process has been killed by a signal, try the next process
				if !matches!(proc.get_state(), State::Running) {
					continue;
				}
				let regs = proc.regs.clone();
				let syscalling = proc.syscalling;
				drop(proc);
				break (Some((pid, proc_mutex)), Some((regs, syscalling)));
			};
			// Set current running process
			sched.curr_proc = proc;
			let tmp_stack = sched.get_tmp_stack();
			(switch_info, tmp_stack)
		};
		unsafe {
			// Unlock interrupt handler
			event::unlock_callbacks(0x20);
			pic::end_of_interrupt(0x0);
			match switch_info {
				// Runnable process found: resume execution
				Some((regs, syscalling)) => regs.switch(!syscalling),
				// No runnable process found: idle
				None => stack::switch(tmp_stack as _, crate::enter_loop),
			}
		}
	}
}

/// Ends the current tick on the current CPU.
///
/// Since this function triggers an interruption, the caller must ensure that no critical mutex is
/// locked, that could be used in the interruption handler. Otherwise, a deadlock could occur.
#[inline]
pub fn end_tick() {
	unsafe {
		asm!("int 0x20");
	}
}
