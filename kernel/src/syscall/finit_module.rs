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

//! The `finit_module` system call allows to load a module on the kernel.

use crate::{
	file::{fd::FileDescriptorTable, perm::AccessProfile},
	module,
	module::Module,
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use core::{alloc::AllocError, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
	vec,
};

pub fn finit_module(
	Args((fd, _param_values, _flags)): Args<(c_int, SyscallString, c_int)>,
	ap: AccessProfile,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	if !ap.is_privileged() {
		return Err(errno!(EPERM));
	}
	// Read file
	let image = fds.lock().get_fd(fd)?.get_file().vfs_entry.read_all()?;
	let module = Module::load(&image)?;
	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno!(EEXIST))
	}
}
