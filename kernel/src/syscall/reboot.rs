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

use crate::{power, process::Process};
use core::ffi::{c_int, c_void};
use macros::syscall;
use utils::{errno, errno::Errno};

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
		if !proc.access_profile.is_privileged() {
			return Err(errno!(EPERM));
		}
	}

	match cmd as u32 {
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
