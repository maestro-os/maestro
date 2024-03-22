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

//! The `delete_module` system call allows to unload a module from the kernel.

use crate::{
	module,
	process::{mem_space::ptr::SyscallString, Process},
};
use core::ffi::c_uint;
use macros::syscall;
use utils::{collections::string::String, errno, errno::Errno};

// TODO handle flags

#[syscall]
pub fn delete_module(name: SyscallString, _flags: c_uint) -> Result<i32, Errno> {
	let name = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		if !proc.access_profile.is_privileged() {
			return Err(errno!(EPERM));
		}

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let name = name.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		String::try_from(name)?
	};

	// TODO handle dependency (don't unload a module that is required by another)
	module::remove(&name);

	Ok(0)
}
