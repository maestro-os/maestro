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

//! Architecture-specific **Hardware Abstraction Layers** (HAL).

use crate::{
	println,
	process::scheduler::{alloc_core_local, init_core_local},
};
use utils::errno::AllocResult;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[macro_use]
pub mod x86;

/// The name of the current CPU architecture.
pub const ARCH: &str = {
	#[cfg(target_arch = "x86")]
	{
		"x86"
	}
	#[cfg(target_arch = "x86_64")]
	{
		"x86_64"
	}
};

/// Tells whether the APIC is present or not.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static mut APIC: bool = false;

/// Architecture-specific initialization, stage 1.
///
/// `first` tells whether we are on the first CPU to boot.
pub(crate) fn init1(first: bool) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		cli();
		if !has_sse() {
			panic!("SSE support is required to run this kernel :(");
		}
		enable_sse();
		// Setup interrupt handlers
		if first {
			idt::init_table();
		}
		idt::bind();
		// Enable GLOBAL flag
		let mut cr4 = register_get!("cr4") | (1 << 7);
		// Enable SMEP and SMAP if supported
		let (smep, smap) = supports_supervisor_prot();
		if smep {
			cr4 |= 1 << 20;
		}
		if smap {
			cr4 |= 1 << 21;
		}
		unsafe {
			register_set!("cr4", cr4);
		}
	}
}

/// Architecture-specific initialization, stage 2.
pub(crate) fn init2() -> AllocResult<()> {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		// Detect APIC
		let apic = apic::is_present();
		unsafe {
			APIC = apic;
		}
		if apic {
			println!("Setup APIC");
			pic::disable();
			apic::init(true)?;
			apic::enumerate_ioapic()?;
			println!("Enumerate CPU cores");
			enumerate_cpus();
		} else {
			println!("No APIC found. Fallback to the PIC");
			pic::enable(0x20, 0x28);
		}

		// Init core-local
		alloc_core_local()?;
		init_core_local();
		gdt::flush();
		tss::init();

		println!("Setup timers");
		sti();
		timer::init(true)?;
	}
	Ok(())
}

/// Returns the ID of the current CPU core.
#[inline]
pub fn core_id() -> u8 {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::apic::lapic_id()
}

/// Enables interruptions on the given IRQ.
pub fn enable_irq(irq: u8) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		if unsafe { APIC } {
			// TODO enable in I/O APIC
		} else {
			pic::enable_irq(irq);
		}
	}
}

/// Disable interruptions on the given IRQ.
pub fn disable_irq(irq: u8) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		if unsafe { APIC } {
			// TODO disable in I/O APIC
		} else {
			pic::disable_irq(irq);
		}
	}
}

/// Sends an End-Of-Interrupt message for the given interrupt `irq`.
pub fn end_of_interrupt(irq: u8) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		if unsafe { APIC } {
			apic::end_of_interrupt();
		} else {
			pic::end_of_interrupt(irq);
		}
	}
}
