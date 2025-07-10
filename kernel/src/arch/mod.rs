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
	arch::x86::{
		apic,
		apic::{IO_APIC_REDIRECTIONS_OFF, ioapic_redirect_count, ioapic_write},
		paging::{FLAG_CACHE_DISABLE, FLAG_GLOBAL, FLAG_WRITE, FLAG_WRITE_THROUGH},
		pic,
	},
	memory::{PhysAddr, vmem::KERNEL_VMEM},
	println,
	process::scheduler::{CPU, Cpu},
	sync::once::OnceInit,
};
use utils::{collections::vec::Vec, limits::PAGE_SIZE};

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
				println!("ent type {}", e.entry_type);
				// FIXME: this relies on entries being sorted by ascending type ID
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
					// Map an I/O APIC's registers
					1 if apic => {
						let ent = unsafe { e.body::<IOAPIC>() };
						let base_addr = PhysAddr(ent.ioapic_address as _).down_align_to(PAGE_SIZE);
						KERNEL_VMEM.lock().map(
							base_addr,
							base_addr.kernel_to_virtual().unwrap(),
							FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH | FLAG_WRITE | FLAG_GLOBAL,
						);
					}
					// Redirect a legacy interrupt
					2 if apic => {
						let ent = unsafe { e.body::<InterruptSourceOverride>() };
						println!("ent {:?}", ent);
						// Find the associated I/O APIC
						let ioapic = madt
							.entries()
							.filter(|e| e.entry_type == 1)
							.map(|e| unsafe { e.body::<IOAPIC>() })
							.find(|ioapic| {
								let gsi = ent.gsi;
								let base_addr = PhysAddr(ioapic.ioapic_address as _);
								let max_entries =
									unsafe { ioapic_redirect_count(base_addr) } as u32;
								(ioapic.gsi..ioapic.gsi + max_entries).contains(&gsi)
							});
						println!("ioapic {:?}", ioapic);
						// Remap the interrupt
						if let Some(ioapic) = ioapic {
							let base_addr = PhysAddr(ioapic.ioapic_address as _);
							let i = (ent.gsi - ioapic.gsi) as u8;
							// TODO flags?
							let val = 0x20 + ent.irq_source as u64;
							unsafe {
								ioapic_write(
									base_addr,
									IO_APIC_REDIRECTIONS_OFF + i * 2,
									val as u32,
								);
								ioapic_write(
									base_addr,
									IO_APIC_REDIRECTIONS_OFF + i * 2 + 1,
									(val >> 32) as u32,
								);
							}
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
