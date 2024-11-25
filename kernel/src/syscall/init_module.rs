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
	file::perm::AccessProfile,
	module,
	module::Module,
	process::{
		mem_space::copy::{SyscallSlice, SyscallString},
		Process,
	},
	syscall::Args,
};
use core::ffi::c_ulong;
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn init_module(
	Args((module_image, len, _param_values)): Args<(SyscallSlice<u8>, c_ulong, SyscallString)>,
	ap: AccessProfile,
) -> EResult<usize> {
	if !ap.is_privileged() {
		return Err(errno!(EPERM));
	}
	let image = module_image
		.copy_from_user_vec(0, len as usize)?
		.ok_or_else(|| errno!(EFAULT))?;
	let module = Module::load(&image)?;
	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno!(EEXIST))
	}
}
