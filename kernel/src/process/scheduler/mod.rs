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
//! Scheduling can be disabled/enabled by entering a **critical section**, with
//! [`preempt_disable`]/[`preempt_enable`], or with [`critical`].

pub mod cpu;
pub mod defer;
pub mod switch;

use crate::{
	arch::{
		core_id,
		x86::{apic, apic::IpiDeliveryMode, cli, idt::IntFrame},
	},
	process::{
		Process, State,
		scheduler::{cpu::per_cpu, switch::switch},
	},
	sync::spin::IntSpin,
	time::{clock::Clock, sleep_for},
};
use core::{
	cmp::Ordering,
	hint::unlikely,
	mem, ptr,
	sync::atomic::Ordering::{Relaxed, Release},
};
use cpu::{CPU, IDLE_CPUS, PerCpu};
use utils::{list_type, ptr::arc::Arc};

/// Flag in the preempt counter, telling whether preemption has been requested
const PREEMPT_FLAG: u32 = 1 << 31;

// TODO must be configurable
/// The timeout, in milliseconds, after which processes are rebalanced
const REBALANCE_TIMEOUT: u64 = 100;

/// Queue of processes to run
struct RunQueue {
	/// Queue of processes to run
	queue: list_type!(Process, sched_node),
	/// The number of processes in queue
	len: usize,
}

/// A process scheduler.
///
/// Each CPU core has its own scheduler.
pub struct Scheduler {
	/// Run queue
	run_queue: IntSpin<RunQueue>,
	/// The currently running process
	cur_proc: IntSpin<Arc<Process>>,

	/// The task used to make the current CPU idle
	idle_task: Arc<Process>,
}

impl Scheduler {
	/// Returns the current running process.
	#[inline]
	pub fn get_current_process(&self) -> Arc<Process> {
		self.cur_proc.lock().clone()
	}

	/// Swaps the current running process for `new`, returning the previous.
	pub fn swap_current_process(&self, new: Arc<Process>) -> Arc<Process> {
		per_cpu()
			.kernel_stack
			.store(new.kernel_stack.top().as_ptr() as _, Release);
		mem::replace(&mut *self.cur_proc.lock(), new)
	}

	/// Tells whether this scheduler can immediately run `proc`
	pub fn can_immediately_run(&self, proc: &Process) -> bool {
		let ord = self.get_current_process().cmp_priority(proc);
		ord == Ordering::Less
	}

	/// Returns the number of processes in the run queue
	#[inline]
	pub fn queue_len(&self) -> usize {
		self.run_queue.lock().len
	}

	/// Selects the next process to run and atomically makes it the current process.
	///
	/// Returns the previous and next processes as raw pointers, for the context switch, or `None`
	/// if the next process is the same as the current one (nothing to do).
	fn pick_and_switch(&self) -> Option<(*const Process, *const Process)> {
		// Held all along to avoid concurrency issues, because the [`rebalance`] task holds it too
		let mut queue = self.run_queue.lock();
		let next = match queue.queue.front() {
			Some(proc) => {
				queue.queue.rotate_left();
				proc
			}
			None => self.idle_task.clone(),
		};
		let mut cur = self.cur_proc.lock();
		// If the process to run is the current, do nothing
		if ptr::eq(next.as_ref(), cur.as_ref()) {
			return None;
		}
		// Update the idle bitmap if necessary
		if cur.is_idle_task() {
			IDLE_CPUS.clear_bit(core_id() as _);
		} else if next.is_idle_task() {
			IDLE_CPUS.set_bit(core_id() as _);
		}
		// Swap current running process. We use pointers to avoid cloning the Arc
		per_cpu()
			.kernel_stack
			.store(next.kernel_stack.top().as_ptr() as _, Release);
		let next_ptr = Arc::as_ptr(&next);
		let prev = mem::replace(&mut *cur, next);
		Some((Arc::as_ptr(&prev), next_ptr))
	}
}

// TODO take into account power-states and core affinity
/// Enqueues `proc` onto a scheduler.
///
/// This function attempts to select the scheduler that is the most suitable for the process, in an
/// attempt to load-balance processes across CPU cores.
pub(crate) fn enqueue(proc: &Arc<Process>) {
	debug_assert_eq!(proc.get_state(), State::Running);
	// If the process already is enqueued, do nothing
	let last_cpu = {
		let links = proc.links.lock();
		if links.cur_cpu.is_some() {
			return;
		}
		links.last_cpu
	};
	// Select the CPU to run the process
	let cpu_cmp = |cpu0: &&PerCpu, cpu1: &&PerCpu| {
		// TODO use other metrics than the amount of running processes
		let cnt0 = cpu0.sched.queue_len();
		let cnt1 = cpu1.sched.queue_len();
		cnt0.cmp(&cnt1)
	};
	// Attempt to run on the last CPU that run the process, if any
	let cpu = last_cpu
		.and_then(|cpu| {
			// Explore the CPU topology to find the closest suitable core
			cpu::topology::find_closest_core(cpu, proc)
		})
		.or_else(|| {
			// Attempt to find an idle CPU
			IDLE_CPUS
				.iter()
				.enumerate()
				.find(|(_, idle)| *idle)
				.map(|(id, _)| &CPU[id])
		})
		.or_else(|| {
			// Select the scheduler with the least running processes among those able to run the
			// process immediately
			CPU.iter()
				.filter(|cpu| cpu.sched.can_immediately_run(proc))
				.min_by(cpu_cmp)
		})
		.or_else(|| {
			// Select the scheduler with the least running processes
			CPU.iter().min_by(cpu_cmp)
		})
		// There is at least one CPU on the system
		.unwrap();
	// FIXME: deadlock
	/*#[cfg(feature = "strace")]
	println!(
		"[strace {}] enqueue on core {}",
		proc.get_pid(),
		cpu.apic_id
	);*/
	// Enqueue
	{
		let mut run_queue = cpu.sched.run_queue.lock();
		run_queue.queue.insert_back(proc.clone());
		run_queue.len += 1;
		let mut links = proc.links.lock();
		debug_assert!(links.cur_cpu.is_none());
		links.cur_cpu = Some(cpu);
		links.last_cpu = Some(cpu);
	}
	// If the process has been enqueued on another core, notify it with a reschedule IPI
	if !ptr::eq(cpu, per_cpu()) {
		apic::ipi(cpu.apic_id, IpiDeliveryMode::Fixed, defer::INT);
	}
}

/// Re-attaches `proc` to the **current** core's run queue.
///
/// Unlike [`enqueue`], this never load-balances the process onto another core. It must only be
/// used for a process that is *currently running on the calling core* (example: it is this core's
/// current process) and that has been dequeued. Using [`enqueue`] in that situation may result in
/// the process running on two cores at once (critical bug).
pub(crate) fn enqueue_current(proc: &Arc<Process>) {
	debug_assert_eq!(proc.get_state(), State::Running);
	let cpu = per_cpu();
	// Lock order is always run_queue then links (see `enqueue`/`dequeue`), to avoid deadlocks
	let mut run_queue = cpu.sched.run_queue.lock();
	let mut links = proc.links.lock();
	// If the process is already enqueued, do nothing
	if let Some(cur) = links.cur_cpu {
		debug_assert!(ptr::eq(cur, cpu));
		return;
	}
	run_queue.queue.insert_back(proc.clone());
	run_queue.len += 1;
	links.cur_cpu = Some(cpu);
	links.last_cpu = Some(cpu);
}

/// Removes the process from its scheduler, if any.
pub(crate) fn dequeue(proc: &Arc<Process>) {
	// If the process is not enqueued, do nothing
	let Some(cpu) = proc.links.lock().cur_cpu else {
		return;
	};
	// FIXME: deadlock
	/*#[cfg(feature = "strace")]
	println!("[strace {}] dequeue", proc.get_pid());*/
	// Remove from queue
	let mut run_queue = cpu.sched.run_queue.lock();
	unsafe {
		run_queue.queue.remove(proc);
	}
	run_queue.len -= 1;
	let mut links = proc.links.lock();
	let prev = links.cur_cpu.take();
	links.last_cpu = prev;
}

/// Attempts to return the CPU cores with the least and most processes queued, without locking
fn min_max() -> (&'static PerCpu, &'static PerCpu) {
	let mut iter = CPU.iter();
	let mut min = iter.next().unwrap(); // The system has at least one core
	let mut max = min;
	let mut min_cnt = min.sched.queue_len();
	let mut max_cnt = min_cnt;
	for cpu in iter {
		let proc_count = cpu.sched.queue_len();
		if proc_count < min_cnt {
			min = cpu;
			min_cnt = proc_count;
		} else if proc_count > max_cnt {
			max = cpu;
			max_cnt = proc_count;
		}
	}
	(min, max)
}

/// Rebalances processes across cores
fn rebalance() {
	/*
	 * This function works by picking the CPUs with the least and most running processes, and
	 * balancing processes across them
	 *
	 * Searching for the least and most loaded CPUs is done without locking. So the result is
	 * not exact, but it does not matter since this function is called in a loop.
	 *
	 * The system tends more and more towards equilibrium at each call
	 */
	let (mut dst, mut src) = min_max();
	if ptr::eq(dst, src) {
		return;
	}
	// Lock both cores' queues
	let mut dst_queue = dst.sched.run_queue.lock();
	let mut src_queue = src.sched.run_queue.lock();
	// Process counts might have changed before we locked
	if dst_queue.len > src_queue.len {
		mem::swap(&mut dst, &mut src);
		mem::swap(&mut dst_queue, &mut src_queue);
	}
	// No need to do anything if no core has more than one process
	if src_queue.len <= 1 {
		return;
	}
	// Compute the number of processes to move
	let diff = src_queue.len - dst_queue.len;
	let mut iter = src_queue.queue.iter();
	let mut migrated_count = 0;
	// We must have more than one process to move, otherwise a process might get needlessly moved
	// back and forth
	for _ in 1..diff {
		let Some(cursor) = iter.next() else {
			break;
		};
		// Skip currently running process
		if ptr::eq(
			cursor.value(),
			Arc::as_ptr(&src.sched.get_current_process()),
		) {
			continue;
		}
		// Remove the process from its old queue
		let proc = cursor.remove();
		#[cfg(feature = "strace")]
		println!(
			"[strace {}] migrate from {} to {}",
			proc.get_pid(),
			src.apic_id,
			dst.apic_id
		);
		// Update the process's scheduler
		{
			let mut links = proc.links.lock();
			links.cur_cpu = Some(dst);
			links.last_cpu = Some(dst);
		}
		// Insert in the new queue
		dst_queue.queue.insert_back(proc);
		migrated_count += 1;
	}
	dst_queue.len += migrated_count;
	src_queue.len -= migrated_count;
}

/// The entry point of the kernel task rebalancing processes across CPU cores
pub(crate) fn rebalance_task() -> ! {
	loop {
		rebalance();
		// Sleep
		let mut remain = 0;
		let _ = sleep_for(Clock::Monotonic, REBALANCE_TIMEOUT * 1_000_000, &mut remain);
	}
}

/// Reschedules, switching context to the next process to run on the current core.
///
/// If no process is ready to run, the scheduler halts the current core until a process becomes
/// runnable.
///
/// **Note**: calling this function inside a critical section is invalid.
pub fn schedule() {
	// Disable interrupts so that no interrupt can occur before switching to the next process
	cli();
	// Reset preempt flag
	#[allow(unused_variables)]
	let old_preempt_counter = per_cpu().preempt_counter.fetch_or(PREEMPT_FLAG, Relaxed);
	// Ensure we are not in a critical section
	debug_assert_eq!(old_preempt_counter & !PREEMPT_FLAG, 0);
	// Make deferred calls
	defer::consume();
	let Some((prev, next)) = per_cpu().sched.pick_and_switch() else {
		return;
	};
	unsafe {
		switch(prev, next);
	}
}

/// Preempt at the next yield point outside a critical section
pub fn preempt() {
	per_cpu().preempt_counter.fetch_and(!PREEMPT_FLAG, Relaxed);
}

/// Enter a critical section, disabling preemption.
#[inline]
pub fn preempt_disable() {
	per_cpu().preempt_counter.fetch_add(1, Relaxed);
}

/// Exit a critical section, enabling preemption if the counter reaches zero.
///
/// The function may reschedule if the counter has reached zero.
///
/// # Safety
///
/// Calling this function outside a critical section is undefined.
pub unsafe fn preempt_enable() {
	let cnt = per_cpu().preempt_counter.fetch_sub(1, Relaxed);
	// If the preemption hasn't been requested yet, the high bit is set, so this condition isn't
	// fulfilled
	if unlikely(cnt == 0) {
		schedule();
	}
}

/// Reschedules, if requested by the timer, and we are not in a critical section.
///
/// This function may never return in case the process has been turned to a zombie after switching
/// to another process.
pub fn preempt_check_resched() {
	let cnt = per_cpu().preempt_counter.load(Relaxed);
	// If the preemption hasn't been requested yet, the high bit is set, so this condition isn't
	// fulfilled
	if unlikely(cnt == 0) {
		schedule();
	}
}

/// Executes `f` in a critical section.
#[inline]
pub fn critical<F: FnOnce() -> T, T>(f: F) -> T {
	preempt_disable();
	let r = f();
	unsafe {
		preempt_enable();
	}
	r
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
	let Some(sig) = proc.signal.lock().next_signal() else {
		return false;
	};
	// Prepare for execution of signal handler
	proc.sig_handlers.lock()[sig.0 as usize].exec(sig, frame);
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
