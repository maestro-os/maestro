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

//! CPU-specific features.

use core::arch::asm;

pub mod sse;

/// Returns the value stored into the specified register.
#[macro_export]
macro_rules! register_get {
	($reg:expr) => {{
		let mut val: u32;
		core::arch::asm!(concat!("mov {}, ", $reg), out(reg) val);
		val
	}};
}

/// Sets the value of the specified register.
#[macro_export]
macro_rules! register_set {
	($reg:expr, $val:expr) => {{
		core::arch::asm!(concat!("mov ", $reg, ", {}"), in(reg) $val);
	}};
}

/// Returns HWCAP bitmask for ELF.
pub fn get_hwcap() -> u32 {
	let mut hwcap: u32;
	unsafe {
		asm!(
			"cpuid",
			in("eax") 1,
			out("ebx") _,
			out("ecx") _,
			lateout("edx") hwcap,
		);
	}
	hwcap
}
