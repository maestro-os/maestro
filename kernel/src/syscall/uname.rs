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

//! The `uname` syscall is used to retrieve information about the system.

use crate::{
	arch::ARCH,
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
	HOSTNAME, NAME, VERSION,
};
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// The length of a field of the utsname structure.
const UTSNAME_LENGTH: usize = 65;

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

pub fn uname(Args(buf): Args<SyscallPtr<Utsname>>) -> EResult<usize> {
	let mut utsname = Utsname {
		sysname: [0; UTSNAME_LENGTH],
		nodename: [0; UTSNAME_LENGTH],
		release: [0; UTSNAME_LENGTH],
		version: [0; UTSNAME_LENGTH],
		machine: [0; UTSNAME_LENGTH],
	};
	utils::slice_copy(NAME.as_bytes(), &mut utsname.sysname);
	utils::slice_copy(&HOSTNAME.lock(), &mut utsname.nodename);
	utils::slice_copy(VERSION.as_bytes(), &mut utsname.release);
	utils::slice_copy(&[], &mut utsname.version);
	utils::slice_copy(ARCH.as_bytes(), &mut utsname.machine);
	buf.copy_to_user(&utsname)?;
	Ok(0)
}
