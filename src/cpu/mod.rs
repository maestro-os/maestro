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
