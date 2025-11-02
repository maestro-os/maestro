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

//! Statistics about memory usage.

use crate::sync::spin::Spin;
use core::{
	fmt,
	fmt::{Display, Formatter},
};

/// Stores memory usage information. Each field is in KiB.
#[derive(Clone)]
pub struct MemInfo {
	/// The total amount of memory on the system.
	pub mem_total: usize,
	/// The total amount of free physical memory.
	pub mem_free: usize,
	/// The total amount of free + reclaimable memory.
	pub mem_available: usize,
	/// The total amount of active (mapped) memory.
	pub active: usize,
	/// The total amount of inactive (not mapped but cached) memory.
	pub inactive: usize,
}

impl Display for MemInfo {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		writeln!(
			f,
			"MemTotal: {} kB
MemFree: {} kB
MemAvailable: {} kB
Active: {} kB
Inactive: {} kB",
			self.mem_total, self.mem_free, self.mem_available, self.active, self.inactive
		)
	}
}

/// Memory usage statistics.
pub static MEM_INFO: Spin<MemInfo> = Spin::new(MemInfo {
	mem_total: 0,
	mem_free: 0,
	mem_available: 0,
	active: 0,
	inactive: 0,
});
