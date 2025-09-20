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

//! Per-CPU structure, bitmaps and CPU topology

pub mod topology;

use super::{RunQueue, Scheduler, defer::DeferredCallQueue};
use crate::{
	arch::x86::{gdt::Gdt, tss::Tss},
	process::{Process, mem_space::MemSpace},
	sync::{atomic::AtomicU64, once::OnceInit, spin::IntSpin},
};
use core::{
	cell::UnsafeCell,
	sync::atomic::{
		AtomicBool, AtomicU32, AtomicUsize,
		Ordering::{Acquire, Release},
	},
};
use topology::TopologyNode;
use utils::{
	collections::vec::Vec,
	errno::{AllocResult, CollectResult},
	list,
	ptr::arc::{AtomicArc, AtomicOptionalArc},
};

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
	/// The pointer stored by this field is returned by `Arc::into_raw`
	pub mem_space: AtomicOptionalArc<MemSpace>,

	/// Queue of deferred calls to be executed on this core
	pub(super) deferred_calls: DeferredCallQueue,
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
				run_queue: IntSpin::new(RunQueue {
					queue: list!(Process, sched_node),
					len: 0,
				}),
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
pub(crate) fn init_cpu_list(mut cpu: Vec<PerCpu>) -> AllocResult<()> {
	// If no CPU is found, just add the current
	if cpu.is_empty() {
		cpu.push(PerCpu::new(0, 0, 0)?)?;
	}
	println!("{} CPU cores found", cpu.len());
	unsafe {
		OnceInit::init(&CPU, cpu);
	}
	let idle_cpus = init_bitmap(true)?;
	unsafe {
		OnceInit::init(&IDLE_CPUS, idle_cpus);
	}
	Ok(())
}

/// Helper allocating an atomic bitmap large enough to have a bit per CPU on the system
///
/// `set`: if `true` all bits are set at the beginning. Else they are clear
pub fn init_bitmap(set: bool) -> AllocResult<Vec<AtomicUsize>> {
	let len = CPU.len().div_ceil(usize::BITS as usize);
	let unit_val = if set { !0 } else { 0 };
	(0..len)
		.map(|_| AtomicUsize::new(unit_val))
		.collect::<CollectResult<_>>()
		.0
}

/// Sets the bit for the given `cpu` in `bitmap`
pub fn bitmap_set(bitmap: &[AtomicUsize], cpu: usize) {
	let unit = cpu / usize::BITS as usize;
	let bit = cpu % usize::BITS as usize;
	bitmap[unit].fetch_or(1 << bit, Release);
}

/// Clears the bit for the given `cpu` in `bitmap`
pub fn bitmap_clear(bitmap: &[AtomicUsize], cpu: usize) {
	let unit = cpu / usize::BITS as usize;
	let bit = cpu % usize::BITS as usize;
	bitmap[unit].fetch_and(!(1 << bit), Release);
}

/// Iterates on bit values for each CPU in `bitmap`
pub fn bitmap_iter(bitmap: &[AtomicUsize]) -> impl Iterator<Item = bool> {
	bitmap
		.iter()
		.flat_map(|unit| {
			let unit = unit.load(Acquire);
			(0..usize::BITS).map(move |bit| unit & (1 << bit) != 0)
		})
		.take(CPU.len())
}

/// Returns an iterator over the IDs of all online CPUs. This is useful for TLB shootdown on all
/// cores
pub fn iter_online() -> impl Iterator<Item = u32> {
	CPU.iter()
		.filter(|cpu| cpu.online.load(Acquire))
		.map(|cpu| cpu.apic_id)
}
