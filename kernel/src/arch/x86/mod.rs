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

//! x86-specific code.
//!
//! Documentation for the x86 architecture is available [here](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html).

pub mod apic;
pub mod gdt;
pub mod idt;
pub mod io;
pub mod paging;
pub mod pic;
pub mod smp;
pub mod timer;
pub mod tss;

use core::arch::asm;

/// MSR: APIC base
pub const IA32_APIC_BASE_MSR: u32 = 0x1b;
/// MSR: FS base
pub const IA32_FS_BASE: u32 = 0xc0000100;
/// MSR: GS base
pub const IA32_GS_BASE: u32 = 0xc0000101;
/// MSR: Kernel GS base
pub const IA32_KERNEL_GS_BASE: u32 = 0xc0000102;

/// Process default `rflags`
pub const DEFAULT_FLAGS: usize = 0x202;
/// Process default `FCW`
pub const DEFAULT_FCW: u32 = 0b1100111111;
/// Process default `MXCSR`
pub const DEFAULT_MXCSR: u32 = 0b1111111000000;

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
pub fn get_rflags() -> usize {
	let mut flags;
	unsafe {
		#[cfg(target_pointer_width = "32")]
		asm!(
			"pushfd",
			"pop {:e}",
			out(reg) flags,
		);
		#[cfg(target_pointer_width = "64")]
		asm!(
			"pushfq",
			"pop {:r}",
			out(reg) flags,
		);
	}
	flags
}

/// Tells whether maskable interruptions are enabled on the current core.
#[inline]
pub fn is_interrupt_enabled() -> bool {
	get_rflags() & 0x200 != 0
}

/// Disables maskable interruptions on the current core.
#[inline(always)]
pub fn cli() {
	unsafe {
		asm!("cli");
	}
}

/// Enables maskable interruptions on the current core.
#[inline(always)]
pub fn sti() {
	unsafe {
		asm!("sti");
	}
}

/// Waits for an interruption on the current core.
#[inline(always)]
pub fn hlt() {
	unsafe {
		asm!("hlt");
	}
}

/// Calls the CPUID instruction.
#[inline]
pub fn cpuid(mut eax: u32, mut ebx: u32, mut ecx: u32, mut edx: u32) -> (u32, u32, u32, u32) {
	unsafe {
		#[cfg(target_arch = "x86")]
		asm!(
			"cpuid",
			inout("eax") eax,
			inout("ebx") ebx,
			inout("ecx") ecx,
			inout("edx") edx,
		);
		#[cfg(target_arch = "x86_64")]
		asm!(
			"push rbx",
			"mov ebx, {rbx:e}",
			"cpuid",
			"mov {rbx:e}, ebx",
			"pop rbx",
			inout("rax") eax,
			rbx = inout(reg) ebx,
			inout("rcx") ecx,
			inout("rdx") edx,
		);
	}
	(eax, ebx, ecx, edx)
}

/// Read value from a Model Specific Register.
#[inline]
pub fn rdmsr(msr: u32) -> u64 {
	let mut edx: u32;
	let mut eax: u32;
	unsafe {
		asm!(
			"rdmsr",
			in("ecx") msr,
			out("edx") edx,
			out("eax") eax,
			options(nostack)
		);
	}
	((edx as u64) << 32) | eax as u64
}

/// Write value to a Model Specific Register.
#[inline]
pub fn wrmsr(msr: u32, val: u64) {
	let edx = (val >> 32) as u32;
	let eax = val as u32;
	unsafe {
		asm!(
			"wrmsr",
			in("ecx") msr,
			in("edx") edx,
			in("eax") eax,
			options(nostack)
		);
	}
}

/// Returns HWCAP bitmask for ELF.
#[inline]
pub fn get_hwcap() -> u32 {
	cpuid(1, 0, 0, 0).3
}

/// Tells whether the CPU supports SSE.
pub fn has_sse() -> bool {
	get_hwcap() & (1 << 25) != 0
}

/// Enables SSE.
pub fn enable_sse() {
	// Enable x87 FPU
	let cr0 = (register_get!("cr0") & !0b100) | 0b10;
	// Enable FXSAVE and FXRSTOR (thus, enabling SSE) and SSE exceptions
	let cr4 = register_get!("cr4") | 0b11000000000;
	unsafe {
		register_set!("cr0", cr0);
		register_set!("cr4", cr4);
	}
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
/// The function returns the previous state of the flag.
///
/// # Safety
///
/// Disabling this feature which makes read-only data writable in kernelspace.
#[inline]
pub unsafe fn set_write_protected(lock: bool) -> bool {
	let mut val = register_get!("cr0");
	let prev = val & (1 << 16) != 0;
	if lock {
		val |= 1 << 16;
	} else {
		val &= !(1 << 16);
	}
	register_set!("cr0", val);
	prev
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

/// FXstate buffer.
#[derive(Clone)]
#[repr(align(16))]
pub struct FxState(pub [u8; 512]);

/// Performs the `fxsave` instruction on `fxstate`.
#[inline]
pub fn fxsave(fxstate: &mut FxState) {
	unsafe {
		asm!("fxsave [{}]", in(reg) fxstate.0.as_mut_ptr());
	}
}

/// Performs the `fxrstor` instruction on `fxstate`.
#[inline]
pub fn fxrstor(fxstate: &FxState) {
	unsafe {
		asm!("fxrstor [{}]", in(reg) fxstate.0.as_ptr());
	}
}
