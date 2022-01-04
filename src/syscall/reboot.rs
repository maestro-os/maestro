//! The `reboot` system call allows the superuser to power off, reboot, halt or suspend the system.

use core::arch::asm;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;

/// First magic number.
const MAGIC: u32 = 0xde145e83;
/// Second magic number.
const MAGIC2: u32 = 0x40367d6e;

/// Command to power off the system.
const CMD_POWEROFF: u32 = 0;
/// Command to reboot the system.
const CMD_REBOOT: u32 = 1;
/// Command to halt the system.
const CMD_HALT: u32 = 2;
/// Command to suspend the system.
const CMD_SUSPEND: u32 = 3;

/// The implementation of the `reboot` syscall.
pub fn reboot(regs: &Regs) -> Result<i32, Errno> {
	let magic = regs.ebx as u32;
	let magic2 = regs.ecx as u32;
	let cmd = regs.edx as u32;

	if magic != MAGIC || magic2 != MAGIC2 {
		return Err(errno::EINVAL);
	}

	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();
		if proc.get_uid() != 0 {
			return Err(errno::EPERM);
		}
	}

	match cmd {
		CMD_POWEROFF => {
			crate::println!("Power down...");
			// TODO Use ACPI to power off the system

			// Loop to avoid compilation error
			loop {}
		},

		CMD_REBOOT => {
			crate::println!("Rebooting...");

			// TODO Use ACPI reset to ensure everything reboots

			// TODO Pulse the keyboard controller's reset pin

			// Triggering a triple fault, causing the system to reboot
			unsafe {
				asm!("jmp $0xffff, $0");
			}

			// In case rebooting didn't work (very unlikely)
			crate::halt();
		},

		CMD_HALT => {
			crate::println!("Halting...");

			// TODO Send a signal to all other cores to stop them
			crate::halt();
		},

		CMD_SUSPEND => {
			// TODO Use ACPI to suspend the system

			Ok(0)
		},

		_ => Err(errno::EINVAL),
	}
}
