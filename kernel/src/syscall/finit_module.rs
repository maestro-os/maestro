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
	module,
	module::Module,
	process::{mem_space::ptr::SyscallString, Process},
};
use core::{alloc::AllocError, ffi::c_int};
use macros::syscall;
use utils::{errno, errno::Errno, io::IO, vec};

#[syscall]
pub fn finit_module(fd: c_int, _param_values: SyscallString, _flags: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let open_file_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		if !proc.access_profile.is_privileged() {
			return Err(errno!(EPERM));
		}

		let fds_mutex = proc.file_descriptors.as_ref().unwrap();
		let fds = fds_mutex.lock();

		fds.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()
			.clone()
	};
	let image = {
		let mut open_file = open_file_mutex.lock();
		let len = open_file.get_size().try_into().map_err(|_| AllocError)?;
		let mut image = vec![0u8; len]?;
		open_file.read(0, image.as_mut_slice())?;
		image
	};

	let module = Module::load(image.as_slice())?;
	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno!(EEXIST))
	}
}
