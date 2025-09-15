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

//! Kernel module system calls.

use crate::{
	file::perm::AccessProfile,
	memory::user::{UserSlice, UserString},
	module,
	module::Module,
	process::Process,
};
use core::{
	ffi::{c_int, c_uint, c_ulong},
	hint::unlikely,
};
use utils::{errno, errno::EResult};

pub fn init_module(
	module_image: *mut u8,
	len: c_ulong,
	_param_values: UserString,
) -> EResult<usize> {
	let module_image = UserSlice::from_user(module_image, len as _)?;
	if unlikely(!AccessProfile::cur_task().is_privileged()) {
		return Err(errno!(EPERM));
	}
	let image = module_image
		.copy_from_user_vec(0)?
		.ok_or_else(|| errno!(EFAULT))?;
	let module = Module::load(&image)?;
	module::add(module)?;
	Ok(0)
}

pub fn finit_module(fd: c_int, _param_values: UserString, _flags: c_int) -> EResult<usize> {
	if unlikely(!AccessProfile::cur_task().is_privileged()) {
		return Err(errno!(EPERM));
	}
	// Read file
	let file = Process::current()
		.file_descriptors()
		.lock()
		.get_fd(fd)?
		.get_file()
		.clone();
	let image = file.read_all()?;
	let module = Module::load(&image)?;
	module::add(module)?;
	Ok(0)
}

// TODO handle flags
pub fn delete_module(name: UserString, _flags: c_uint) -> EResult<usize> {
	if unlikely(!AccessProfile::cur_task().is_privileged()) {
		return Err(errno!(EPERM));
	}
	let name = name.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	// TODO handle dependency (don't unload a module that is required by another)
	module::remove(&name)?;
	Ok(0)
}
