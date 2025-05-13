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

//! Host management system calls.

use crate::{
	NAME, VERSION,
	arch::ARCH,
	file::perm::AccessProfile,
	memory::user::{UserPtr, UserSlice},
	power,
	syscall::Args,
};
use core::{
	ffi::{c_int, c_void},
	hint::unlikely,
};
use utils::{errno, errno::EResult, limits::HOST_NAME_MAX, slice_copy};

/// The length of a field of the utsname structure.
const UTSNAME_LENGTH: usize = 65;

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

/// Userspace structure storing uname information.
#[repr(C)]
#[derive(Debug)]
pub struct Utsname {
	/// Operating system name.
	sysname: [u8; UTSNAME_LENGTH],
	/// Network node hostname.
	nodename: [u8; UTSNAME_LENGTH],
	/// Operating system release.
	release: [u8; UTSNAME_LENGTH],
	/// Operating system version.
	version: [u8; UTSNAME_LENGTH],
	/// Hardware identifier.
	machine: [u8; UTSNAME_LENGTH],
}

pub fn uname(Args(buf): Args<UserPtr<Utsname>>) -> EResult<usize> {
	let mut utsname = Utsname {
		sysname: [0; UTSNAME_LENGTH],
		nodename: [0; UTSNAME_LENGTH],
		release: [0; UTSNAME_LENGTH],
		version: [0; UTSNAME_LENGTH],
		machine: [0; UTSNAME_LENGTH],
	};
	slice_copy(NAME.as_bytes(), &mut utsname.sysname);
	slice_copy(&crate::HOSTNAME.lock(), &mut utsname.nodename);
	slice_copy(VERSION.as_bytes(), &mut utsname.release);
	slice_copy(&[], &mut utsname.version);
	slice_copy(ARCH.as_bytes(), &mut utsname.machine);
	buf.copy_to_user(&utsname)?;
	Ok(0)
}

pub fn sethostname(
	Args((name, len)): Args<(*mut u8, usize)>,
	ap: AccessProfile,
) -> EResult<usize> {
	// Check the size of the hostname is in bounds
	if unlikely(len > HOST_NAME_MAX) {
		return Err(errno!(EINVAL));
	}
	// Check permission
	if !ap.is_privileged() {
		return Err(errno!(EPERM));
	}
	// Copy
	let name = UserSlice::from_user(name, len)?;
	let mut hostname = crate::HOSTNAME.lock();
	*hostname = name.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	Ok(0)
}

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
