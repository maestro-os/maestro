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

//! Symmetric MultiProcessing management.

use super::apic::lapic_id;
use crate::process::scheduler::Cpu;

/// Initializes the SMP.
///
/// `cpu` is the list of CPU cores on the system.
pub fn init(cpu: &[Cpu]) {
	let lapic_id = lapic_id();
	// TODO copy trampoline code
	for cpu in cpu {
		// Do no attempt to boot the current core
		if cpu.apic_id == lapic_id {
			continue;
		}
		// TODO send INIT IPI
		// TODO send startup IPI
	}
}
