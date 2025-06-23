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

//! Architecture-specific **Hardware Abstraction Layers** (HAL).

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[macro_use]
pub mod x86;

/// The name of the current CPU architecture.
pub const ARCH: &str = {
	#[cfg(target_arch = "x86")]
	{
		"x86"
	}
	#[cfg(target_arch = "x86_64")]
	{
		"x86_64"
	}
};

/// Tells whether the APIC is present or not.
static mut APIC: bool = false;

/// Architecture-specific initialization.
pub fn init() {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		if !has_sse() {
			panic!("SSE support is required to run this kernel :(");
		}
		enable_sse();
		cli();
		let apic = apic::is_present();
		unsafe {
			APIC = apic;
		}
		if apic::is_present() {
			pic::disable();
			apic::init();
		} else {
			pic::enable(0x20, 0x28);
		}
		idt::init();
	}
}

/// Sends an End-Of-Interrupt message for the given interrupt `irq`.
pub fn end_of_interrupt(irq: u8) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		use x86::*;
		let apic = unsafe { APIC };
		if apic {
			apic::end_of_interrupt();
		} else {
			pic::end_of_interrupt(irq);
		}
	}
}
