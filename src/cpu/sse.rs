//! This module implements SSE-related features.

/// Tells whether the CPU has SSE.
pub fn is_present() -> bool {
	unsafe {
		super::cpuid_has_sse()
	}
}
