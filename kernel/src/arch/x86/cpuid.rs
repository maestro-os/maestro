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

//! CPUID instruction utilities

use core::arch::asm;

/// Vendor string: AMD
pub const CPUID_VENDOR_INTEL: &[u8; 12] = b"GenuineIntel";
/// Vendor string: Intel
pub const CPUID_VENDOR_AMD: &[u8; 12] = b"AuthenticAMD";

/// Calls the CPUID instruction.
#[inline]
pub fn cpuid(mut eax: u32, mut ecx: u32) -> (u32, u32, u32, u32) {
	let mut ebx;
	let mut edx;
	unsafe {
		#[cfg(target_arch = "x86")]
		asm!(
			"cpuid",
			inout("eax") eax,
			out("ebx") ebx,
			inout("ecx") ecx,
			out("edx") edx,
		);
		#[cfg(target_arch = "x86_64")]
		asm!(
			"push rbx",
			"cpuid",
			"mov {ebx:e}, ebx",
			"pop rbx",
			inout("eax") eax,
			ebx = out(reg) ebx,
			inout("ecx") ecx,
			out("edx") edx,
		);
	}
	(eax, ebx, ecx, edx)
}

/// Retrieves the current CPU's vendor string
pub fn vendor() -> [u8; 12] {
	let (_, ebx, ecx, edx) = cpuid(0, 0);
	let mut vendor = [0; 12];
	vendor[..4].copy_from_slice(&ebx.to_ne_bytes());
	vendor[4..8].copy_from_slice(&edx.to_ne_bytes());
	vendor[8..].copy_from_slice(&ecx.to_ne_bytes());
	vendor
}

/// Returns the maximum supported base leaf (below `0x80000000`)
#[inline]
pub fn base_max_leaf() -> u32 {
	cpuid(0, 0).0
}

/// Returns the maximum support base leaf (`0x80000000` or above)
#[inline]
pub fn extended_max_leaf() -> u32 {
	cpuid(0x80000000, 0).0
}

/// Returns whether the leaf `0x4` is available
#[inline]
pub fn has_leaf_0x4() -> bool {
	if base_max_leaf() >= 0x4 {
		cpuid(0x4, 0).0 != 0
	} else {
		false
	}
}

/// Returns whether the leaf `0xb` is available
#[inline]
pub fn has_leaf_0xb() -> bool {
	if base_max_leaf() >= 0xb {
		cpuid(0xb, 0).1 != 0
	} else {
		false
	}
}

/// Returns whether there could be several cores in the package
#[inline]
pub fn has_package_bits() -> bool {
	let edx = cpuid(1, 0).3;
	edx & (1 << 28) != 0
}
