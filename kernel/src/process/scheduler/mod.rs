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
	process::{Process, State, mem_space::MemSpace, pid::Pid, scheduler::switch::switch},
	sync::{
		atomic::AtomicU64,
		once::OnceInit,
		rwlock::{IntRwLock, RwLock},
	},
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

	/// The core's scheduler.
	pub scheduler: Scheduler,

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

/// The list of CPU cores on the system.
pub static CPU: OnceInit<Vec<Cpu>> = unsafe { OnceInit::new() };
/// The list of core-local structures. There is one per CPU.
pub static CORE_LOCAL: OnceInit<Vec<CoreLocal>> = unsafe { OnceInit::new() };

/// Initializes schedulers.
pub fn init() -> AllocResult<()> {
	let idle_task = Process::idle_task()?;
	// Initialize core locales
	let core_locals = CPU
		.iter()
		.map(|cpu| CoreLocal {
			kernel_stack: AtomicUsize::new(0),
			user_stack: AtomicUsize::new(0),

			cpu,
			gdt: Default::default(),
			tss: Default::default(),

			scheduler: Scheduler {
				total_ticks: AtomicU64::new(0),

				queue: RwLock::new(BTreeMap::new()),
				cur_proc: AtomicArc::from(idle_task.clone()),

				idle_task: idle_task.clone(),
			},

			mem_space: AtomicOptionalArc::new(),
		})
		.collect::<CollectResult<Vec<CoreLocal>>>()
		.0?;
	unsafe {
		OnceInit::init(&CORE_LOCAL, core_locals);
	}
	init_core_local();
	gdt::flush();
	tss::init();
	// Boot other cores
	smp::init(&CPU)?;
	Ok(())
}

/// A process scheduler.
///
/// Each CPU core has its own scheduler.
pub struct Scheduler {
	/// The total number of ticks since the instantiation of the scheduler
	total_ticks: AtomicU64,

	/// Queue of processes to run
	queue: IntRwLock<BTreeMap<Pid, Arc<Process>>>,
	/// The currently running process
	cur_proc: AtomicArc<Process>,

	/// The task used to make the current CPU idle
	idle_task: Arc<Process>,
}

impl Scheduler {
	/// Returns the total number of ticks since the instantiation of the
	/// scheduler.
	pub fn get_total_ticks(&self) -> u64 {
		self.total_ticks.load(Relaxed)
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

	/// Adds a process to the scheduler's queue.
	#[inline]
	pub fn enqueue(&self, proc: Arc<Process>) -> AllocResult<()> {
		self.queue.write().insert(*proc.pid, proc)?;
		Ok(())
	}

	/// Removes the process with the given `pid` from the scheduler's queue.
	///
	/// If the process is not attached to this scheduler, the function does nothing.
	#[inline]
	pub fn dequeue(&self, pid: Pid) {
		self.queue.write().remove(&pid);
	}

	/// Returns the next process to run with its PID.
	///
	/// `cur_pid` is the PID of the currently running process
	fn get_next_process(&self, cur_pid: Pid) -> Option<Arc<Process>> {
		// Get the current process, or take the first process in the list if no
		// process is running
		let process_filter =
			|(_, proc): &(&Pid, &Arc<Process>)| proc.get_state() == State::Running;
		let processes = self.queue.read();
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

// TODO take into account power-states, NUMA, process priority and core affinity
/// Enqueues `proc` onto a scheduler.
///
/// This function attempts to select the scheduler that is the most suitable for the process, in an
/// attempt to load-balance processes across CPU cores.
pub fn enqueue(_proc: &Process) {
	// select a scheduler to run the process onto
	todo!()
}

/// Runs the scheduler. Switching context to the next process to run on the current core.
///
/// If no process is ready to run, the scheduler halts the current core until a process becomes
/// runnable.
pub fn schedule() {
	// Disable interrupts so that no interrupt can occur before switching to the next process
	cli();
	let sched = &core_local().scheduler;
	sched.total_ticks.fetch_add(1, Relaxed);
	let cur_pid = sched.cur_proc.get().get_pid();
	let (prev, next) = {
		// Find the next process to run
		let next = sched
			.get_next_process(cur_pid)
			.unwrap_or_else(|| sched.idle_task.clone());
		// If the process to run is the current, do nothing
		if next.get_pid() == cur_pid {
			return;
		}
		// Swap current running process. We use pointers to avoid cloning the Arc
		let next_ptr = Arc::as_ptr(&next);
		let prev = sched.swap_current_process(next);
		(Arc::as_ptr(&prev), next_ptr)
	};
	// Send end of interrupt, so that the next tick can be received
	end_of_interrupt(0);
	unsafe {
		switch(prev, next);
	}
}

/// Runs the scheduler if the timeslice of the current process is over.
///
/// This function may never return in case the process has been turned to a zombie after switching
/// to another process.
pub fn may_schedule() {
	todo!()
}

/// Returns `false` if the execution shall continue. Else, the execution shall be paused.
fn alter_flow_impl(frame: &mut IntFrame) -> bool {
	// Disable interruptions to prevent execution from being stopped before the reference to
	// `Process` is dropped
	cli();
	// If the process is not running anymore, stop execution
	let proc = Process::current();
	if proc.get_state() != State::Running {
		return true;
	}
	// Get signal handler to execute, if any
	let (sig, handler) = {
		let mut signal_manager = proc.signal.lock();
		let Some(sig) = signal_manager.next_signal() else {
			return false;
		};
		let handler = signal_manager.handlers.lock()[sig as usize].clone();
		(sig, handler)
	};
	// Prepare for execution of signal handler
	handler.exec(sig, &proc, frame);
	// If the process is still running, continue execution
	proc.get_state() != State::Running
}

/// Before returning to userspace from the current context, this function checks the state of the
/// current process to potentially alter the execution flow.
///
/// Arguments:
/// - `ring` is the ring the current context is returning to.
/// - `frame` is the interrupt frame.
///
/// The execution flow can be altered by:
/// - The process is no longer in [`State::Running`] state
/// - A signal handler has to be executed
///
/// This function disables interruptions.
///
/// This function never returns in case the process state turns to [`State::Zombie`].
pub fn alter_flow(ring: u8, frame: &mut IntFrame) {
	// If returning to kernelspace, do nothing
	if ring < 3 {
		return;
	}
	// Use a separate function to drop everything, since `schedule` may never return
	if alter_flow_impl(frame) {
		schedule();
	}
}
