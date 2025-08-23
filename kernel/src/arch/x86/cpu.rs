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

//! CPU topology

use crate::{
	acpi,
	acpi::madt::{Madt, ProcessorLocalApic},
	arch::x86::cpuid::{
		CPUID_VENDOR_AMD, CPUID_VENDOR_INTEL, cpuid, extended_max_leaf, has_leaf_0x4,
		has_leaf_0xb, has_package_bits,
	},
	println,
	process::scheduler::{CPU, CPU_TOPOLOGY, PerCpu, per_cpu},
	sync::once::OnceInit,
};
use utils::{collections::vec::Vec, errno::AllocResult};

/// Enumerates CPUs on the system using ACPI.
///
/// This function **must not** be called if there is no APIC on the system.
pub fn enumerate_cpus() -> AllocResult<()> {
	let mut cpu = Vec::new();
	if let Some(madt) = acpi::get_table::<Madt>() {
		// Register CPU cores
		let iter = madt
			.entries()
			.filter(|e| e.entry_type == 0)
			.map(|e| unsafe { e.body::<ProcessorLocalApic>() })
			.filter(|e| e.apic_flags & 0b11 != 0);
		for e in iter {
			cpu.push(PerCpu::new(e.processor_id, e.apic_id as _, e.apic_flags)?)?;
		}
	}
	// If no CPU is found, just add the current
	if cpu.is_empty() {
		cpu.push(PerCpu::new(1, 0, 0)?)?;
	}
	println!("{} CPU cores found", cpu.len());
	unsafe {
		OnceInit::init(&CPU, cpu);
	}
	Ok(())
}

/// Adds the current CPU to the topology tree ([`CPU_TOPOLOGY`])
pub fn topology_add() -> AllocResult<()> {
	let ebx = cpuid(1, 0).1;
	let mut lapic_id = (ebx >> 24) & 0xff;
	let bits_below_package = ((ebx >> 16) as u8 - 1).trailing_ones();
	let has_package_bits = has_package_bits();
	let cpu = &CPU[lapic_id as usize];
	match &*per_cpu().vendor {
		CPUID_VENDOR_INTEL if has_leaf_0xb() => {
			let mut depth = 0;
			let mut bits = 0;
			loop {
				let (eax, _, ecx, _) = cpuid(0xb, depth);
				if (ecx >> 8) & 0xff == 0 {
					break;
				}
				depth += 1;
				bits += eax & 0x1f;
			}
			let mut parent = &CPU_TOPOLOGY;
			for i in (0..depth).rev() {
				let (eax, _, _, edx) = cpuid(0xb, depth);
				let current_bits = eax & 0x1f;
				lapic_id = edx;
				bits -= current_bits;
				parent = parent.insert(
					(lapic_id >> bits) & ((1 << (32 - current_bits)) - 1),
					(i == 0).then_some(cpu),
				)?;
			}
		}
		CPUID_VENDOR_INTEL if has_package_bits && has_leaf_0x4() => {
			let eax = cpuid(0x4, 0).0;
			let bits_for_core = 32 - ((eax >> 26) & 0x3f).leading_zeros();
			let bits_for_thread = bits_below_package - bits_for_core;

			let core_mask = (1 << bits_for_core) - 1;
			let thread_mask = (1 << bits_for_thread) - 1;
			let package = CPU_TOPOLOGY.insert(lapic_id >> bits_below_package, None)?;
			let core = package.insert((lapic_id >> bits_for_thread) & core_mask, None)?;
			core.insert(lapic_id & thread_mask, Some(cpu))?;
		}
		CPUID_VENDOR_AMD if has_package_bits && extended_max_leaf() >= 0x80000008 => {
			let ecx = cpuid(0x80000008, 0).2;
			let apic_id_size = (ecx >> 12) & 0xf;
			let bits_for_core = if apic_id_size > 0 {
				apic_id_size
			} else {
				32 - (ecx & 0xff).leading_zeros()
			};
			let bits_for_thread = bits_below_package - bits_for_core;

			let core_mask = (1 << bits_for_core) - 1;
			let thread_mask = (1 << bits_for_thread) - 1;
			let package = CPU_TOPOLOGY.insert(lapic_id >> bits_below_package, None)?;
			let core = package.insert((lapic_id >> bits_for_thread) & core_mask, None)?;
			core.insert(lapic_id & thread_mask, Some(cpu))?;
		}
		_ if has_package_bits => {
			let package = CPU_TOPOLOGY.insert(lapic_id >> bits_below_package, None)?;
			let core = package.insert(lapic_id & ((1 << bits_below_package) - 1), None)?;
			core.insert(0, Some(cpu))?;
		}
		_ => {
			CPU_TOPOLOGY.insert(lapic_id as _, Some(cpu))?;
		}
	}
	Ok(())
}
