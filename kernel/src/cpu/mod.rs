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
	($reg:expr) => {
		unsafe {
			let mut val: usize;
			core::arch::asm!(concat!("mov {}, ", $reg), out(reg) val);
			val
		}
	};
}

/// Sets the value of the specified register.
#[macro_export]
macro_rules! register_set {
	($reg:expr, $val:expr) => {{
		core::arch::asm!(concat!("mov ", $reg, ", {}"), in(reg) $val);
	}};
}

/// Returns the value of the RFLAGS register.
#[inline]
pub fn get_rflags() -> u32 {
	let mut flags;
	unsafe {
		asm!("pushf", "pop {}", out(reg) flags);
	}
	flags
}

/// Calls the CPUID instruction.
#[inline]
pub fn cpuid(mut eax: u32, mut ebx: u32, mut ecx: u32, mut edx: u32) -> (u32, u32, u32, u32) {
	unsafe {
		asm!(
			"cpuid",
			inout("eax") eax,
			inout("ebx") ebx,
			inout("ecx") ecx,
			inout("edx") edx,
		);
	}
	(eax, ebx, ecx, edx)
}

/// Returns HWCAP bitmask for ELF.
#[inline]
pub fn get_hwcap() -> u32 {
	cpuid(1, 0, 0, 0).3
}

/// Tells whether SMEP and SMAP are supported (in that order).
#[inline]
pub fn supports_supervisor_prot() -> (bool, bool) {
	let (_, flags, ..) = cpuid(7, 0, 0, 0);
	let smep = flags & (1 << 7) != 0;
	let smap = flags & (1 << 20) != 0;
	(smep, smap)
}

/// Sets whether the kernel can write to read-only pages.
///
/// # Safety
///
/// Disabling this feature which makes read-only data writable in kernelspace.
#[inline]
pub unsafe fn set_write_protected(lock: bool) {
	let mut val = register_get!("cr0");
	if lock {
		val |= 1 << 16;
	} else {
		val &= !(1 << 16);
	}
	register_set!("cr0", val);
}

/// Sets or clears the AC flag to disable or enable SMAP.
///
/// # Safety
///
/// SMAP provides a security against potentially malicious data accesses. As such, it should be
/// disabled only when strictly necessary.
///
/// Enabling SMAP removes access to memory addresses that were previously accessible. It is the
/// caller's responsibility to ensure no invalid memory accesses are done afterward.
#[inline]
pub unsafe fn set_smap_enabled(enabled: bool) {
	// TODO cache in RAM instead
	let (_, smap) = supports_supervisor_prot();
	if !smap {
		return;
	}
	if enabled {
		asm!("clac");
	} else {
		asm!("stac");
	}
}
