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

//! CPU interruptions help functions.

use core::arch::asm;

/// Tells whether interrupts are enabled on the current CPU kernel.
pub fn is_interrupt_enabled() -> bool {
	let mut flags: usize;
	unsafe {
		#[cfg(target_pointer_width = "32")]
		asm!(
			"pushfd",
			"pop {flags:e}",
			flags = out(reg) flags,
		);
		#[cfg(target_pointer_width = "64")]
		asm!(
			"pushfq",
			"pop {flags:r}",
			flags = out(reg) flags,
		);
	}
	flags & 0x200 != 0
}

/// Disables interruptions on the current CPU kernel.
#[inline(always)]
pub fn cli() {
	unsafe {
		asm!("cli");
	}
}

/// Enables interruptions on the current CPU kernel.
#[inline(always)]
pub fn sti() {
	unsafe {
		asm!("sti");
	}
}

/// Waits for an interruption on the current CPU kernel.
#[inline(always)]
pub fn hlt() {
	unsafe {
		asm!("hlt");
	}
}
