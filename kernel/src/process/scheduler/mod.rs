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
//! TODO

pub mod switch;

use crate::{
	arch::x86::{cli, idt::IntFrame, pic},
	event,
	event::{CallbackHook, CallbackResult},
	process::{pid::Pid, scheduler::switch::switch, Process, State},
	sync::{atomic::AtomicU64, mutex::IntMutex, once::OnceInit},
	time,
};
use core::{
	mem,
	sync::{
		atomic,
		atomic::{AtomicUsize, Ordering::Release},
	},
};
use utils::{
	collections::btreemap::{BTreeMap, MapIterator},
	errno::AllocResult,
	math::rational::Rational,
	ptr::arc::Arc,
};

/// The process scheduler.
pub static SCHEDULER: OnceInit<IntMutex<Scheduler>> = unsafe { OnceInit::new() };

/// Initializes schedulers.
pub fn init() -> AllocResult<()> {
	unsafe {
		SCHEDULER.init(IntMutex::new(Scheduler::new()?));
	}
	SCHEDULER.get().lock().setup_gs_base();
	Ok(())
}

/// Kernel CPU local storage.
#[derive(Default)]
#[repr(C)]
pub struct KernelGs {
	/// The current kernel stack.
	pub kernel_stack: AtomicUsize,
	/// The stashed user stack.
	pub user_stack: AtomicUsize,
}

/// A process scheduler.
///
/// Each CPU core has its own scheduler.
pub struct Scheduler {
	/// The ticking callback hook, called at a regular interval to make the
	/// scheduler work.
	tick_callback_hook: CallbackHook,
	/// The total number of ticks since the instantiation of the scheduler.
	total_ticks: AtomicU64,

	/// A binary tree containing all processes registered to the current
	/// scheduler.
	processes: BTreeMap<Pid, Arc<Process>>,
	/// The process currently being executed by the scheduler's core.
	curr_proc: Arc<Process>,
	/// The current number of processes in running state.
	running_procs: usize,

	/// The task used to idle.
	idle_task: Arc<Process>,

	/// CPU local storage.
	pub gs: KernelGs,
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub(super) fn new() -> AllocResult<Self> {
		// Register tick callback
		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();
		let tick_callback_hook = event::register_callback(
			pit.get_interrupt_vector(),
			|_: u32, _: u32, _: &mut IntFrame, _: u8| {
				Scheduler::tick();
				CallbackResult::Continue
			},
		)?
		.unwrap();
		let idle_task = Process::idle_task()?;
		Ok(Self {
			tick_callback_hook,
			total_ticks: AtomicU64::new(0),

			processes: BTreeMap::new(),
			curr_proc: idle_task.clone(),
			running_procs: 0,

			idle_task,

			gs: KernelGs::default(),
		})
	}

	/// Sets the GS base on the current core.
	#[inline]
	fn setup_gs_base(&self) {
		#[cfg(target_arch = "x86_64")]
		{
			use crate::arch::x86;
			// Set to `IA32_GS_BASE` instead of `IA32_KERNEL_GS_BASE` since it will get swapped
			// when switching to userspace
			x86::wrmsr(x86::IA32_GS_BASE, &self.gs as *const _ as u64);
		}
	}

	/// Returns the total number of ticks since the instantiation of the
	/// scheduler.
	pub fn get_total_ticks(&self) -> u64 {
		self.total_ticks.load(atomic::Ordering::Relaxed)
	}

	/// Returns an iterator on the scheduler's processes.
	pub fn iter_process(&self) -> MapIterator<'_, Pid, Arc<Process>> {
		self.processes.iter()
	}

	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(&self, pid: Pid) -> Option<Arc<Process>> {
		Some(self.processes.get(&pid)?.clone())
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(&self, _tid: Pid) -> Option<Arc<Process>> {
		todo!()
	}

	/// Returns the current running process.
	pub fn get_current_process(&self) -> Arc<Process> {
		self.curr_proc.clone()
	}

	/// Swaps the current running process for `new`, returning the previous.
	pub fn swap_current_process(&mut self, new: Arc<Process>) -> Arc<Process> {
		self.gs
			.kernel_stack
			.store(new.kernel_stack.top().as_ptr() as _, Release);
		mem::replace(&mut self.curr_proc, new)
	}

	/// Adds a process to the scheduler.
	pub fn add_process(&mut self, process: Process) -> AllocResult<Arc<Process>> {
		if process.get_state() == State::Running {
			self.increment_running();
		}
		let pid = process.pid.get();
		let ptr = Arc::new(process)?;
		self.processes.insert(pid, ptr.clone())?;
		Ok(ptr)
	}

	/// Removes the process with the given pid `pid`.
	pub fn remove_process(&mut self, pid: Pid) {
		let Some(proc) = self.get_by_pid(pid) else {
			return;
		};
		if proc.get_state() == State::Running {
			self.decrement_running();
		}
		self.processes.remove(&pid);
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
		if self.running_procs >= 1 {
			pit.set_frequency(self.get_ticking_frequency());
			pit.set_enabled(true);
		}
	}

	/// Decrements the number of running processes.
	pub fn decrement_running(&mut self) {
		self.running_procs -= 1;
		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();
		if self.running_procs == 0 {
			pit.set_enabled(false);
		} else {
			pit.set_frequency(self.get_ticking_frequency());
		}
	}

	/// Returns the next process to run with its PID.
	fn get_next_process(&self) -> Option<Arc<Process>> {
		// Get the current process, or take the first process in the list if no
		// process is running
		let curr_pid = self.curr_proc.get_pid();
		let process_filter =
			|(_, proc): &(&Pid, &Arc<Process>)| matches!(proc.get_state(), State::Running);
		self.processes
			.range((curr_pid + 1)..)
			.find(process_filter)
			.or_else(|| {
				// If no suitable process is found, go back to the beginning to check processes
				// located before the previous process (looping)
				self.processes.range(..=curr_pid).find(process_filter)
			})
			.map(|(_, proc)| proc.clone())
	}

	/// Ticking the scheduler.
	///
	/// The function looks for the next process to run, then switches context to it.
	///
	/// If no process is ready to run, the scheduler halts the current core until a process becomes
	/// runnable.
	pub fn tick() {
		// Disable interrupts so that no interrupt can occur before switching to the next process
		cli();
		let (prev, next) = {
			let mut sched = SCHEDULER.get().lock();
			sched.total_ticks.fetch_add(1, atomic::Ordering::Relaxed);
			// Find the next process to run
			let next = sched.get_next_process().unwrap_or(sched.idle_task.clone());
			// If the process to run is the current, do nothing
			if next.get_pid() == sched.curr_proc.get_pid() {
				return;
			}
			// Swap current running process. We use pointers to avoid cloning the Arc
			let next_ptr = Arc::as_ptr(&next);
			let prev = sched.swap_current_process(next);
			(Arc::as_ptr(&prev), next_ptr)
		};
		// Send end of interrupt, so that the next tick can be received
		pic::end_of_interrupt(0);
		unsafe {
			switch(prev, next);
		}
	}
}
