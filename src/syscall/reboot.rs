//! The `reboot` system call allows the superuser to power off, reboot, halt or
//! suspend the system.

use crate::errno;
use crate::errno::Errno;
use crate::process::Process;
use core::arch::asm;
use core::ffi::c_int;
use core::ffi::c_void;
use macros::syscall;

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

#[syscall]
pub fn reboot(magic: c_int, magic2: c_int, cmd: c_int, _arg: *const c_void) -> Result<i32, Errno> {
	if (magic as u32) != MAGIC || (magic2 as u32) != MAGIC2 {
		return Err(errno!(EINVAL));
	}

	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		if proc.uid != 0 {
			return Err(errno!(EPERM));
		}
	}

	match cmd as u32 {
		CMD_POWEROFF => {
			crate::println!("Power down...");

			// TODO Use ACPI to power off the system

			// In case power down didn't work (very unlikely)
			crate::halt();
		}

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
		}

		CMD_HALT => {
			crate::println!("Halting...");

			// TODO Send a signal to all other cores to stop them
			crate::halt();
		}

		CMD_SUSPEND => {
			// TODO Use ACPI to suspend the system

			Ok(0)
		}

		_ => Err(errno!(EINVAL)),
	}
}
