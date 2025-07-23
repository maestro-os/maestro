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

pub mod switch;

use crate::{
	arch::{
		end_of_interrupt,
		x86::{cli, gdt, gdt::Gdt, idt::IntFrame, smp, tss, tss::Tss},
	},
	event,
	event::{CallbackHook, CallbackResult},
	process::{Process, State, mem_space::MemSpace, pid::Pid, scheduler::switch::switch},
	sync::{
		atomic::AtomicU64,
		once::OnceInit,
		rwlock::{IntReadGuard, IntRwLock, RwLock},
	},
	time,
};
use core::{
	cell::UnsafeCell,
	sync::atomic::{
		AtomicUsize,
		Ordering::{Relaxed, Release},
	},
};
use utils::{
	collections::{btreemap::BTreeMap, vec::Vec},
	errno::{AllocResult, CollectResult},
	ptr::arc::{Arc, AtomicArc, AtomicOptionalArc},
};

/// Description of a CPU core.
pub struct Cpu {
	/// Processor ID
	pub id: u8,
	/// Local APIC ID
	pub apic_id: u8,
	/// Local APIC flags
	pub apic_flags: u32,
}

// This structure is using the C representation because field offsets are important for assembly
/// Kernel core-local storage.
#[repr(C)]
pub struct CoreLocal {
	/// The current kernel stack
	pub kernel_stack: AtomicUsize,
	/// The stashed user stack
	pub user_stack: AtomicUsize,

	/// CPU information
	pub cpu: &'static Cpu,
	/// The CPU's GDT
	pub gdt: Gdt,
	/// The CPU's TSS
	tss: UnsafeCell<Tss>,

	/// Attached memory space
	///
	/// The pointer stored by this field is returned by [`Arc::into_raw`].
	pub mem_space: AtomicOptionalArc<MemSpace>,
}

impl CoreLocal {
	/// Returns a mutable reference to the TSS.
	///
	/// # Safety
	///
	/// Concurrent accesses are undefined.
	#[inline]
	#[allow(clippy::mut_from_ref)]
	pub unsafe fn tss(&self) -> &mut Tss {
		&mut *self.tss.get()
	}
}

/// Returns the core-local structure for the current core.
#[inline]
pub fn core_local() -> &'static CoreLocal {
	#[cfg(target_arch = "x86")]
	{
		use crate::arch::x86::apic::lapic_id;

		&CORE_LOCAL[lapic_id() as usize]
	}
	#[cfg(target_arch = "x86_64")]
	{
		use crate::{arch::x86, memory::VirtAddr};

		let base = x86::rdmsr(x86::IA32_GS_BASE);
		unsafe {
			let ptr = VirtAddr(base as _).as_ptr::<CoreLocal>();
			&*ptr
		}
	}
}

/// Sets, on the current CPU core, the register to make the associated [`CoreLocal`] structure
/// available.
pub(crate) fn init_core_local() {
	#[cfg(target_arch = "x86_64")]
	{
		use crate::arch::{x86, x86::apic::lapic_id};
		let local = &CORE_LOCAL[lapic_id() as usize];
		// Set to `IA32_GS_BASE` instead of `IA32_KERNEL_GS_BASE` since it will get swapped
		// when switching to userspace
		x86::wrmsr(x86::IA32_GS_BASE, local as *const _ as u64);
	}
}

/// The list of CPU cores on the system.
pub static CPU: OnceInit<Vec<Cpu>> = unsafe { OnceInit::new() };
/// The list of core-local structures. There is one per CPU.
pub static CORE_LOCAL: OnceInit<Vec<CoreLocal>> = unsafe { OnceInit::new() };

/// The process scheduler.
pub static SCHEDULER: OnceInit<Scheduler> = unsafe { OnceInit::new() };

/// Initializes schedulers.
pub fn init() -> AllocResult<()> {
	// Initialize core locales
	let core_locals = CPU
		.iter()
		.map(|cpu| CoreLocal {
			kernel_stack: AtomicUsize::new(0),
			user_stack: AtomicUsize::new(0),

			cpu,
			gdt: Default::default(),
			tss: Default::default(),

			mem_space: AtomicOptionalArc::new(),
		})
		.collect::<CollectResult<Vec<CoreLocal>>>()
		.0?;
	unsafe {
		OnceInit::init(&CORE_LOCAL, core_locals);
	}
	// Init the current core's scheduler
	init_core_local();
	gdt::flush();
	tss::init();
	let mut clocks = time::hw::CLOCKS.lock();
	let pit = clocks.get_mut(b"pit".as_slice()).unwrap();
	let tick_callback_hook = event::register_callback(
		pit.get_interrupt_vector(),
		|_: u32, _: u32, _: &mut IntFrame, _: u8| {
			schedule();
			CallbackResult::Continue
		},
	)?
	.unwrap();
	let idle_task = Process::idle_task()?;
	let scheduler = Scheduler {
		tick_callback_hook,
		total_ticks: AtomicU64::new(0),

		processes: RwLock::new(BTreeMap::new()),
		cur_proc: AtomicArc::from(idle_task.clone()),
		running_procs: AtomicUsize::new(0),

		idle_task,
	};
	unsafe {
		OnceInit::init(&SCHEDULER, scheduler);
	}
	// Boot other cores
	smp::init(&CPU)?;
	Ok(())
}

/// Returns the current ticking frequency of the scheduler.
///
/// `running` is the number of running processes.
fn get_ticking_frequency(running: usize) -> u32 {
	(10 * running) as _
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
	processes: IntRwLock<BTreeMap<Pid, Arc<Process>>>,
	/// The process currently being executed by the scheduler's core.
	cur_proc: AtomicArc<Process>,
	/// The current number of processes in running state.
	running_procs: AtomicUsize,

	/// The task used to idle.
	idle_task: Arc<Process>,
}

impl Scheduler {
	/// Returns the total number of ticks since the instantiation of the
	/// scheduler.
	pub fn get_total_ticks(&self) -> u64 {
		self.total_ticks.load(Relaxed)
	}

	/// Returns the current number of processes on the scheduler.
	#[inline]
	pub fn processes_count(&self) -> usize {
		self.processes.read().len()
	}

	/// Read-locks the list of processes, and returns the guard.
	pub fn processes(&self) -> IntReadGuard<'_, BTreeMap<Pid, Arc<Process>>> {
		self.processes.read()
	}

	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(&self, pid: Pid) -> Option<Arc<Process>> {
		self.processes.write().get(&pid).cloned()
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(&self, _tid: Pid) -> Option<Arc<Process>> {
		todo!()
	}

	/// Returns the current running process.
	#[inline]
	pub fn get_current_process(&self) -> Arc<Process> {
		self.cur_proc.get()
	}

	/// Swaps the current running process for `new`, returning the previous.
	pub fn swap_current_process(&self, new: Arc<Process>) -> Arc<Process> {
		core_local()
			.kernel_stack
			.store(new.kernel_stack.top().as_ptr() as _, Release);
		self.cur_proc.replace(new)
	}

	/// Adds a process to the scheduler.
	pub fn add_process(&self, proc: Arc<Process>) -> AllocResult<()> {
		if proc.get_state() == State::Running {
			self.increment_running();
		}
		self.processes.write().insert(*proc.pid, proc)?;
		Ok(())
	}

	/// Removes the process with the given pid `pid`.
	///
	/// If the process is not attached to this scheduler, the function does nothing.
	pub fn remove_process(&self, pid: Pid) {
		let proc = self.processes.write().remove(&pid);
		if let Some(proc) = proc {
			if proc.get_state() == State::Running {
				self.decrement_running();
			}
		}
	}

	/// Increments the number of running processes.
	pub fn increment_running(&self) {
		let running = self.running_procs.fetch_add(1, Release) + 1;
		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();
		if running >= 1 {
			pit.set_frequency(get_ticking_frequency(running));
			pit.set_enabled(true);
		}
	}

	/// Decrements the number of running processes.
	pub fn decrement_running(&self) {
		let running = self.running_procs.fetch_sub(1, Release) - 1;
		let mut clocks = time::hw::CLOCKS.lock();
		let pit = clocks.get_mut(b"pit".as_slice()).unwrap();
		if running == 0 {
			pit.set_enabled(false);
		} else {
			pit.set_frequency(get_ticking_frequency(running));
		}
	}

	/// Returns the next process to run with its PID.
	///
	/// `cur_pid` is the PID of the currently running process
	fn get_next_process(&self, cur_pid: Pid) -> Option<Arc<Process>> {
		// Get the current process, or take the first process in the list if no
		// process is running
		let process_filter =
			|(_, proc): &(&Pid, &Arc<Process>)| matches!(proc.get_state(), State::Running);
		let processes = self.processes.read();
		processes
			.range((cur_pid + 1)..)
			.find(process_filter)
			.or_else(|| {
				// If no suitable process is found, go back to the beginning to check processes
				// located before the previous process (looping)
				processes.range(..=cur_pid).find(process_filter)
			})
			.map(|(_, proc)| proc.clone())
	}
}

/// Runs the scheduler. Switching context to the next process to run on the current core.
///
/// If no process is ready to run, the scheduler halts the current core until a process becomes
/// runnable.
pub fn schedule() {
	// Disable interrupts so that no interrupt can occur before switching to the next process
	cli();
	SCHEDULER.total_ticks.fetch_add(1, Relaxed);
	let cur_pid = SCHEDULER.cur_proc.get().get_pid();
	let (prev, next) = {
		// Find the next process to run
		let next = SCHEDULER
			.get_next_process(cur_pid)
			.unwrap_or_else(|| SCHEDULER.idle_task.clone());
		// If the process to run is the current, do nothing
		if next.get_pid() == cur_pid {
			return;
		}
		// Swap current running process. We use pointers to avoid cloning the Arc
		let next_ptr = Arc::as_ptr(&next);
		let prev = SCHEDULER.swap_current_process(next);
		(Arc::as_ptr(&prev), next_ptr)
	};
	// Send end of interrupt, so that the next tick can be received
	end_of_interrupt(0);
	unsafe {
		switch(prev, next);
	}
}
