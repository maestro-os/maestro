//! CPU-specific features.

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

extern "C" {
	/// Tells whether the CPU has SSE.
	fn cpuid_has_sse() -> bool;

	/// Returns HWCAP bitmask for ELF.
	pub fn get_hwcap() -> u32;
}
