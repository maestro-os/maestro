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
	acpi,
	acpi::madt::{IOAPIC, InterruptSourceOverride, ProcessorLocalApic},
	arch::x86::{apic, pic},
	println,
	process::scheduler::{CPU, Cpu},
	sync::once::OnceInit,
};
use utils::collections::vec::Vec;

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
pub(crate) fn init1() {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		cli();
		if !has_sse() {
			panic!("SSE support is required to run this kernel :(");
		}
		enable_sse();
		idt::init();
	}
}

/// Architecture-specific initialization, stage 2.
pub(crate) fn init2() {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		// Detect APIC
		let apic = apic::is_present();
		unsafe {
			APIC = apic;
		}
		if apic {
			pic::disable();
			apic::init();
		} else {
			pic::enable(0x20, 0x28);
		}
		// List CPUs with ACPI
		let mut cpu = Vec::new();
		if let Some(madt) = acpi::get_madt() {
			// Register CPU cores
			for e in madt.entries() {
				// TODO do a match, to remap the APIC
				println!("ent type {}", e.entry_type);
				match e.entry_type {
					// Register a new CPU
					0 => {
						let ent = unsafe { e.body::<ProcessorLocalApic>() };
						if ent.apic_flags & 0b11 == 0 {
							continue;
						}
						cpu.push(Cpu {
							id: ent.processor_id,
							apic_id: ent.apic_id,
							apic_flags: ent.apic_flags,
						})
						.expect("could not insert CPU");
					}
					1 => {
						if apic {
							let ent = unsafe { e.body::<IOAPIC>() };
							println!("{:?}", ent);
							// TODO
						}
					}
					2 => {
						if apic {
							let ent = unsafe { e.body::<InterruptSourceOverride>() };
							println!("{:?}", ent);
							// TODO
						}
					}
					_ => {}
				}
			}
		}
		// If no CPU is found, just add the current
		if cpu.is_empty() {
			cpu.push(Cpu {
				id: 1,
				apic_id: 0,
				apic_flags: 0,
			})
			.expect("could not insert CPU");
		}
		println!("{} CPU cores found", cpu.len());
		unsafe {
			OnceInit::init(&CPU, cpu);
		}
	}
}

/// Sends an End-Of-Interrupt message for the given interrupt `irq`.
pub fn end_of_interrupt(irq: u8) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		let apic = unsafe { APIC };
		if apic {
			apic::end_of_interrupt();
		} else {
			pic::end_of_interrupt(irq);
		}
	}
}
