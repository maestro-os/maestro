//! SSE-related features.

use crate::{cpu::get_hwcap, register_get, register_set};

/// Tells whether the CPU supports SSE.
pub fn is_present() -> bool {
	get_hwcap() & (1 << 25) != 0
}

/// Enables SSE.
pub fn enable() {
	unsafe {
		// Enable x87 FPU
		let cr0 = (register_get!("cr0") & !0b100) | 0b10;
		register_set!("cr0", cr0);

		// Enable FXSAVE and FXRSTOR (thus, enabling SSE) and SSE exceptions
		let cr4 = register_get!("cr4") | 0b11000000000;
		register_set!("cr4", cr4);
	}
}
