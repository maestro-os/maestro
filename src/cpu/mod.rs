//! This module implements CPU-specific features.

pub mod sse;

use core::ffi::c_void;

extern "C" {
	/// Tells whether the CPU has SSE.
	fn cpuid_has_sse() -> bool;

	/// Returns HWCAP bitmask for ELF.
	pub fn get_hwcap() -> u32;

	/// Returns the content of the %cr0 register.
	pub fn cr0_get() -> u32;
	/// Sets the given flags in the %cr0 register.
	pub fn cr0_set(flags: u32);
	/// Clears the given flags in the %cr0 register.
	pub fn cr0_clear(flags: u32);
	/// Returns the content of the %cr2 register.
	pub fn cr2_get() -> *const c_void;
	/// Returns the content of the %cr3 register.
	pub fn cr3_get() -> *mut c_void;
	/// Returns the content of the %cr4 register.
	pub fn cr4_get() -> u32;
	/// Sets the content of the %cr4 register.
	pub fn cr4_set(flags: u32);
}
