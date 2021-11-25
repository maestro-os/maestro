//! This moduule implements CPU-specific features.

extern "C" {
	/// Tells whether the CPU has SSE.
	fn cpuid_has_sse() -> bool;
}

/// Tells whether the CPU has SSE.
pub fn has_sse() -> bool {
	unsafe {
		cpuid_has_sse()
	}
}
