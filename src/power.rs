//! This module handles system power.

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
    // TODO Use ACPI reset to ensure everything reboots
    // TODO Pulse the keyboard controller's reset pin

    // Triggering a triple fault
    unsafe {
        asm!("jmp 0xffff, 0");
    }
    // Halt in case that didn't work
    halt();
}