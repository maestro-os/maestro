//! This module implements SSE-related features.

/// Tells whether the CPU has SSE.
pub fn is_present() -> bool {
	unsafe {
		super::cpuid_has_sse()
	}
}

/// Enables SSE.
pub fn enable() {
	unsafe {
		super::cr0_clear(0b100); // Enable x87 FPU
		super::cr0_set(0b10);

		// Enable FXSAVE and FXRSTOR (thus, enabling SSE) and SSE exceptions
		let cr4 = super::cr4_get() | 0b11000000000;
		super::cr4_set(cr4);
	}
}
