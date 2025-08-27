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

pub mod defer;
pub mod switch;

use crate::{
	arch::{
		core_id, end_of_interrupt,
		x86::{cli, gdt::Gdt, idt::IntFrame, tss::Tss},
	},
	process::{
		Process, State,
		mem_space::MemSpace,
		scheduler::{defer::DeferredCallQueue, switch::switch},
	},
	sync::{
		atomic::AtomicU64,
		mutex::{IntMutex, Mutex},
		once::OnceInit,
	},
};
use core::{
	cell::UnsafeCell,
	cmp::Ordering,
	hint::unlikely,
	ptr,
	sync::atomic::{
		AtomicBool, AtomicU32, AtomicUsize,
		Ordering::{Acquire, Relaxed, Release},
	},
};
use utils::{
	boxed::Box,
	collections::vec::Vec,
	errno::{AllocResult, CollectResult},
	list, list_type,
	ptr::arc::{Arc, AtomicArc, AtomicOptionalArc},
};

/// Helper allocating an atomic bitmap large enough to have a bit per CPU on the system
///
/// `set`: if `true` all bits are set at the beginning. Else they are clear
pub fn init_cpu_bitmap(set: bool) -> AllocResult<Vec<AtomicUsize>> {
	let len = CPU.len().div_ceil(usize::BITS as usize);
	let unit_val = if set { !0 } else { 0 };
	(0..len)
		.map(|_| AtomicUsize::new(unit_val))
		.collect::<CollectResult<_>>()
		.0
}

/// Sets the bit for the given `cpu` in `bitmap`
pub fn cpu_bitmap_set(bitmap: &[AtomicUsize], cpu: usize) {
	let unit = cpu / usize::BITS as usize;
	let bit = cpu % usize::BITS as usize;
	bitmap[unit].fetch_or(1 << bit, Release);
}

/// Clears the bit for the given `cpu` in `bitmap`
pub fn cpu_bitmap_clear(bitmap: &[AtomicUsize], cpu: usize) {
	let unit = cpu / usize::BITS as usize;
	let bit = cpu % usize::BITS as usize;
	bitmap[unit].fetch_and(!(1 << bit), Release);
}

/// Iterates on bit values for each CPU in `bitmap`
pub fn cpu_bitmap_iter(bitmap: &[AtomicUsize]) -> impl Iterator<Item = bool> {
	bitmap
		.iter()
		.flat_map(|unit| {
			let unit = unit.load(Acquire);
			(0..usize::BITS).map(move |bit| unit & (1 << bit) != 0)
		})
		.take(CPU.len())
}

// TODO allow to declare per-core variables everywhere in the codebase using a dedicated ELF
// section

/// Per-CPU data.
///
/// This structure is using `#[repr(C)]` because some field offsets are important for assembly code
#[repr(C)]
pub struct PerCpu {
	/// The current kernel stack
	pub kernel_stack: AtomicUsize,
	/// The stashed user stack
	pub user_stack: AtomicUsize,

	/// Processor ID
	pub cpu_id: u8,
	/// Local APIC ID
	pub apic_id: u32,
	/// Local APIC flags
	pub apic_flags: u32,

	/// Tells whether the CPU core has booted.
	pub online: AtomicBool,
	/// CPU's vendor ID
	pub vendor: OnceInit<[u8; 12]>,

	/// The core's topology node
	pub topology_node: OnceInit<&'static TopologyNode>,

	/// The CPU's GDT
	pub gdt: Gdt,
	/// The CPU's TSS
	tss: UnsafeCell<Tss>,

	/// The core's scheduler
	pub sched: Scheduler,
	/// The time in between each tick on the core, in nanoseconds
	pub tick_period: AtomicU64,
	/// Counter for nested critical sections
	///
	/// The highest bit is used to tell whether preemption has been requested by the timer (clear
	/// = requested, set = not requested)
	pub preempt_counter: AtomicU32,

	/// Attached memory space
	///
	/// The pointer stored by this field is returned by [`Arc::into_raw`]
	pub mem_space: AtomicOptionalArc<MemSpace>,

	/// Queue of deferred calls to be executed on this core
	deferred_calls: DeferredCallQueue,
}

impl PerCpu {
	/// Creates a new instance.
	pub fn new(cpu_id: u8, apic_id: u32, apic_flags: u32) -> AllocResult<Self> {
		let idle_task = Process::idle_task()?;
		Ok(Self {
			kernel_stack: AtomicUsize::new(0),
			user_stack: AtomicUsize::new(0),

			cpu_id,
			apic_id,
			apic_flags,

			online: AtomicBool::new(false),
			vendor: unsafe { OnceInit::new() },

			topology_node: unsafe { OnceInit::new() },

			gdt: Default::default(),
			tss: Default::default(),

			sched: Scheduler {
				queue: IntMutex::new(list!(Process, sched_node)),
				queue_len: AtomicUsize::new(0),
				cur_proc: AtomicArc::from(idle_task.clone()),

				idle_task: idle_task.clone(),
			},
			tick_period: AtomicU64::new(0),
			preempt_counter: AtomicU32::new(1 << 31),

			mem_space: AtomicOptionalArc::new(),

			deferred_calls: DeferredCallQueue::new(),
		})
	}

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

/// Sets, on the current CPU core, the register to make the associated [`PerCpu`] structure
/// available.
pub(crate) fn store_per_cpu() {
	#[cfg(target_arch = "x86_64")]
	{
		use crate::arch::{core_id, x86};
		let local = &CPU[core_id() as usize];
		// Set to `IA32_GS_BASE` instead of `IA32_KERNEL_GS_BASE` since it will get swapped
		// when switching to userspace
		x86::wrmsr(x86::IA32_GS_BASE, local as *const _ as u64);
	}
}

/// Returns the per-CPU structure for the current core.
#[inline]
pub fn per_cpu() -> &'static PerCpu {
	#[cfg(target_arch = "x86")]
	{
		use crate::arch::core_id;
		&CPU[core_id() as usize]
	}
	#[cfg(target_arch = "x86_64")]
	{
		use crate::{arch::x86, memory::VirtAddr};
		let base = x86::rdmsr(x86::IA32_GS_BASE);
		unsafe {
			let ptr = VirtAddr(base as _).as_ptr::<PerCpu>();
			&*ptr
		}
	}
}

/// The list of core-local structures. There is one per CPU.
pub static CPU: OnceInit<Vec<PerCpu>> = unsafe { OnceInit::new() };
/// Bitmap of currently idle CPUs, atomically updated
pub static IDLE_CPUS: OnceInit<Vec<AtomicUsize>> = unsafe { OnceInit::new() };

/// Initializes the CPU list
///
/// This function must be called only once at boot
pub(crate) fn init_cpu(mut cpu: Vec<PerCpu>) -> AllocResult<()> {
	// If no CPU is found, just add the current
	if cpu.is_empty() {
		cpu.push(PerCpu::new(0, 0, 0)?)?;
	}
	println!("{} CPU cores found", cpu.len());
	unsafe {
		OnceInit::init(&CPU, cpu);
	}
	let idle_cpus = init_cpu_bitmap(true)?;
	unsafe {
		OnceInit::init(&IDLE_CPUS, idle_cpus);
	}
	Ok(())
}

/// Returns an iterator over the IDs of all online CPUs. This is useful for TLB shootdown on all
/// cores
pub fn cpu_iter_online() -> impl Iterator<Item = u32> {
	CPU.iter()
		.filter(|cpu| cpu.online.load(Acquire))
		.map(|cpu| cpu.apic_id)
}

/// CPU topology node
pub struct TopologyNode {
	/// Parent node
	parent: Option<&'static TopologyNode>,
	/// Child nodes
	children: UnsafeCell<Vec<&'static TopologyNode>>,

	/// ID used to avoid duplicate entries when this tree is built
	id: u32,
	/// The node's CPU. This is set only if the node is a leaf
	cpu: Option<&'static PerCpu>,
}

unsafe impl Sync for TopologyNode {}

impl TopologyNode {
	/// Inserts a node in the topology tree. This function is thread-safe.
	///
	/// This function must be used only during boot.
	///
	/// On success, the function returns a reference to the node.
	pub(crate) fn insert(
		&'static self,
		id: u32,
		cpu: Option<&'static PerCpu>,
	) -> AllocResult<&'static Self> {
		// Lock to prevent several cores from adding their topology at the same time
		static LOCK: Mutex<()> = Mutex::new(());
		let _guard = LOCK.lock();
		let children = unsafe { &mut *self.children.get() };
		// Looks for a node with the same ID
		if let Some(node) = children.iter().find(|node| node.id == id) {
			// There is already a node, no need to insert a new one
			return Ok(node);
		}
		// Insert node
		let node = Box::new(TopologyNode {
			parent: Some(self),
			children: UnsafeCell::new(Vec::new()),

			id,
			cpu,
		})?;
		let node = Box::into_raw(node);
		unsafe {
			let node = &*node;
			// Link back
			if let Some(cpu) = node.cpu {
				OnceInit::init(&cpu.topology_node, node);
			}
			children.push(node)?;
			Ok(node)
		}
	}

	/// Returns the parent node if any
	#[inline]
	pub fn parent(&self) -> Option<&Self> {
		self.parent
	}

	/// Returns the list of child nodes
	#[inline]
	pub fn children(&self) -> &[&Self] {
		unsafe { &*self.children.get() }
	}
}

/// Tree representing the topology of CPU cores
pub static CPU_TOPOLOGY: TopologyNode = TopologyNode {
	parent: None,
	children: UnsafeCell::new(Vec::new()),

	id: 0,
	cpu: None,
};

/// A process scheduler.
///
/// Each CPU core has its own scheduler.
pub struct Scheduler {
	/// Queue of processes to run
	queue: IntMutex<list_type!(Process, sched_node)>,
	/// The number of processes in queue
	queue_len: AtomicUsize,
	/// The currently running process
	cur_proc: AtomicArc<Process>,

	/// The task used to make the current CPU idle
	idle_task: Arc<Process>,
}

impl Scheduler {
	/// Returns the current running process.
	#[inline]
	pub fn get_current_process(&self) -> Arc<Process> {
		self.cur_proc.get()
	}

	/// Swaps the current running process for `new`, returning the previous.
	pub fn swap_current_process(&self, new: Arc<Process>) -> Arc<Process> {
		per_cpu()
			.kernel_stack
			.store(new.kernel_stack.top().as_ptr() as _, Release);
		self.cur_proc.replace(new)
	}

	/// Returns the next process to run with its PID.
	///
	/// If no process is left to run, the function returns `None`.
	fn get_next_process(&self) -> Option<Arc<Process>> {
		let mut queue = self.queue.lock();
		let proc = queue.front()?;
		queue.rotate_left();
		Some(proc)
	}
}

// TODO take into account power-states and core affinity
/// Enqueues `proc` onto a scheduler.
///
/// This function attempts to select the scheduler that is the most suitable for the process, in an
/// attempt to load-balance processes across CPU cores.
pub fn enqueue(proc: &Arc<Process>) {
	// If the process already is enqueued, do nothing
	let mut links = proc.links.lock();
	if links.cur_cpu.is_some() {
		return;
	}
	// Select the CPU to run the process
	let cpu = links
		// Attempt to run on the last CPU that run the process, if any
		.last_cpu
		.and_then(|cpu| {
			// If the CPU is able to execute immediately, select it
			let ord = cpu.sched.get_current_process().cmp_priority(proc);
			(ord == Ordering::Less).then_some(cpu)
			// TODO else, attempt to explore the CPU topology to find a core that can
		})
		.or_else(|| {
			// Attempt to find an idle CPU
			cpu_bitmap_iter(&IDLE_CPUS)
				.enumerate()
				.find(|(_, idle)| *idle)
				.map(|(id, _)| &CPU[id])
		})
		// TODO use other metrics than the amount of running processes, and attempt to find cores
		// that can run the process immediately first
		.or_else(|| {
			// Select the scheduler with the least running processes
			CPU.iter().min_by(|cpu0, cpu1| {
				let count0 = cpu0.sched.queue_len.load(Acquire);
				let count1 = cpu1.sched.queue_len.load(Acquire);
				count0.cmp(&count1)
			})
		})
		// There is at least one CPU on the system
		.unwrap();
	// Enqueue
	#[cfg(feature = "strace")]
	println!(
		"[strace {}] enqueue on core {}",
		proc.get_pid(),
		cpu.apic_id
	);
	let mut queue = cpu.sched.queue.lock();
	queue.insert_back(proc.clone());
	cpu.sched.queue_len.fetch_add(1, Release);
	links.cur_cpu = Some(cpu);
	links.last_cpu = Some(cpu);
}

/// Removes the process from its scheduler, if any.
pub fn dequeue(proc: &Arc<Process>) {
	// If the process is not enqueued, do nothing
	let mut links = proc.links.lock();
	let Some(cpu) = links.cur_cpu else {
		return;
	};
	// Remove from queue
	#[cfg(feature = "strace")]
	println!("[strace {}] dequeue", proc.get_pid());
	let mut queue = cpu.sched.queue.lock();
	unsafe {
		queue.remove(proc);
	}
	cpu.sched.queue_len.fetch_sub(1, Release);
	let prev = links.cur_cpu.take();
	links.last_cpu = prev;
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
	per_cpu().preempt_counter.fetch_or(1 << 31, Relaxed);
	// Make deferred calls
	defer::consume();
	let sched = &per_cpu().sched;
	let (prev, next) = {
		let prev = sched.cur_proc.get();
		// Find the next process to run
		let next = sched
			.get_next_process()
			.unwrap_or_else(|| sched.idle_task.clone());
		// If the process to run is the current, do nothing
		if ptr::eq(next.as_ref(), prev.as_ref()) {
			return;
		}
		// Update the idle bitmap if necessary
		if prev.is_idle_task() {
			cpu_bitmap_clear(&IDLE_CPUS, core_id() as _);
		} else if next.is_idle_task() {
			cpu_bitmap_set(&IDLE_CPUS, core_id() as _);
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
