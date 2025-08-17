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

//! Program execution routines.
//!
//! Program execution is done in several stages:
//! - Read the program
//! - Parse the program
//! - Build the memory image according to the program
//! - Replace the process's memory with the newly created image to run it

pub mod elf;
pub mod vdso;

use crate::{
	arch::x86::idt::IntFrame,
	file::vfs::ResolutionSettings,
	memory::VirtAddr,
	process::{Process, mem_space::MemSpace, scheduler::per_cpu},
	sync::mutex::Mutex,
};
use utils::{
	collections::{string::String, vec::Vec},
	errno::EResult,
	ptr::arc::Arc,
};

/// Information to prepare a program image to be executed.
pub struct ExecInfo<'s> {
	/// Path resolution settings.
	pub path_resolution: &'s ResolutionSettings,
	/// The list of arguments.
	pub argv: Vec<String>,
	/// The list of environment variables.
	pub envp: Vec<String>,
}

/// A built program image.
pub struct ProgramImage {
	/// The image's memory space.
	mem_space: Arc<MemSpace>,
	/// Tells whether the program runs in compatibility mode.
	compat: bool,

	/// A pointer to the entry point of the program.
	entry_point: VirtAddr,
	/// A pointer to the initial value of the user stack pointer.
	user_stack: VirtAddr,
}

/// Executes the program image `image` on the process `proc`.
///
/// `frame` is the interrupt frame of the current content. The function sets the appropriate values
/// for each register so that the execution beings when the interrupt handler returns.
pub fn exec(proc: &Process, frame: &mut IntFrame, image: ProgramImage) -> EResult<()> {
	// Preform all fallible operations first before touching the process
	let fds = proc
		.file_descriptors
		.as_ref()
		.map(|fds_mutex| -> EResult<_> {
			let fds = fds_mutex.lock();
			let new_fds = fds.duplicate(true)?;
			Ok(Arc::new(Mutex::new(new_fds))?)
		})
		.transpose()?;
	let signal_handlers = Arc::new(Default::default())?;
	// All fallible operations succeeded, flush to process
	MemSpace::bind(&image.mem_space);
	// Safe because no other thread can execute this function at the same time for the same process
	unsafe {
		*proc.file_descriptors.get_mut() = fds;
		*proc.mem_space.get_mut() = Some(image.mem_space);
	}
	// Reset signals
	{
		let mut signal_manager = proc.signal.lock();
		signal_manager.handlers = signal_handlers;
		signal_manager.sigpending = Default::default();
	}
	proc.vfork_wake();
	*proc.tls.lock() = Default::default();
	// Set TSS here for the first process to be executed
	unsafe {
		per_cpu()
			.tss()
			.set_kernel_stack(proc.kernel_stack.top().as_ptr());
	}
	// Set the process's registers
	IntFrame::exec(frame, image.entry_point.0, image.user_stack.0, image.compat);
	#[cfg(target_arch = "x86_64")]
	{
		use crate::{
			arch::{x86, x86::idt::wrap_disable_interrupts},
			process::scheduler::{per_cpu, store_per_cpu},
		};
		use core::{arch::asm, sync::atomic::Ordering::Relaxed};

		// Disable interrupts to prevent data races on `gs`
		wrap_disable_interrupts(|| {
			// Reset segment selector
			unsafe {
				asm!(
					"mov fs, {r:x}",
					"mov gs, {r:x}",
					r = in(reg) 0
				);
			}
			// Reset MSR
			x86::wrmsr(x86::IA32_FS_BASE, 0);
			x86::wrmsr(x86::IA32_KERNEL_GS_BASE, 0);
			store_per_cpu();
			// Update user stack
			per_cpu().user_stack.store(image.user_stack.0 as _, Relaxed);
		});
	}
	Ok(())
}
