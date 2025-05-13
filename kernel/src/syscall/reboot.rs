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

//! The `reboot` system call allows the superuser to power off, reboot, halt or
//! suspend the system.

use crate::{file::perm::AccessProfile, power, syscall::Args};
use core::ffi::{c_int, c_void};
use utils::{errno, errno::EResult};

/// First magic number.
const MAGIC: c_int = 0xde145e83u32 as _;
/// Second magic number.
const MAGIC2: c_int = 0x40367d6eu32 as _;

/// Command to power off the system.
const CMD_POWEROFF: c_int = 0;
/// Command to reboot the system.
const CMD_REBOOT: c_int = 1;
/// Command to halt the system.
const CMD_HALT: c_int = 2;
/// Command to suspend the system.
const CMD_SUSPEND: c_int = 3;

pub fn reboot(
	Args((magic, magic2, cmd, _arg)): Args<(c_int, c_int, c_int, *const c_void)>,
	ap: AccessProfile,
) -> EResult<usize> {
	// Validation
	if magic != MAGIC || magic2 != MAGIC2 {
		return Err(errno!(EINVAL));
	}
	if !ap.is_privileged() {
		return Err(errno!(EPERM));
	}
	// Debug commands: shutdown with QEMU
	#[cfg(config_debug_qemu)]
	{
		use crate::debug::qemu;
		match cmd {
			-1 => qemu::exit(qemu::SUCCESS),
			-2 => qemu::exit(qemu::FAILURE),
			_ => {}
		}
	}
	match cmd {
		CMD_POWEROFF => {
			crate::println!("Power down...");
			power::shutdown();
		}
		CMD_REBOOT => {
			crate::println!("Rebooting...");
			power::reboot();
		}
		CMD_HALT => {
			crate::println!("Halting...");
			power::halt();
		}
		CMD_SUSPEND => {
			// TODO Use ACPI to suspend the system
			todo!()
		}
		_ => Err(errno!(EINVAL)),
	}
}
