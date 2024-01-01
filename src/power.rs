//! This module handles system power.

use crate::io;
use core::arch::asm;

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
	cli!();

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
