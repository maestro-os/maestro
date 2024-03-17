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

//! The `init_module` system call allows to load a module on the kernel.

use crate::{
	module,
	module::Module,
	process::{
		mem_space::ptr::{SyscallSlice, SyscallString},
		Process,
	},
};
use core::ffi::c_ulong;
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn init_module(
	module_image: SyscallSlice<u8>,
	len: c_ulong,
	_param_values: SyscallString,
) -> Result<i32, Errno> {
	let module = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		if !proc.access_profile.is_privileged() {
			return Err(errno!(EPERM));
		}

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let image = module_image
			.get(&mem_space_guard, len as usize)?
			.ok_or_else(|| errno!(EFAULT))?;

		Module::load(image)?
	};

	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno!(EEXIST))
	}
}
