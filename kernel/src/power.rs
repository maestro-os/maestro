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

//! This module handles system power.

use crate::io;
use core::arch::asm;
use utils::interrupt::cli;

/// Halts the kernel until reboot.
pub fn halt() -> ! {
	// TODO Send a signal to all other cores to stop them
	loop {
		unsafe {
			asm!("cli", "hlt");
		}
	}
}

/// Powers the system down.
pub fn shutdown() -> ! {
	// TODO Use ACPI to power off the system
	todo!()
}

/// Reboots the system.
pub fn reboot() -> ! {
	cli();

	// First try: ACPI
	// TODO Use ACPI reset to ensure everything reboots

	// Second try: PS/2
	loop {
		let tmp = unsafe { io::inb(0x64) };
		// Empty keyboard buffer
		if tmp & 0b1 != 0 {
			unsafe {
				io::inb(0x60);
			}
		}
		// If buffer is empty, break
		if tmp & 0b10 == 0 {
			break;
		}
	}
	// PS/2 CPU reset command
	unsafe {
		io::outb(0x64, 0xfe);
	}

	// Third try: triple fault
	unsafe {
		asm!("jmp 0xffff, 0");
	}

	// Giving up
	halt();
}
