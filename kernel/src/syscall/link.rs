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

//! The link system call allows to create a directory.

use crate::{
	file::path::Path,
	process::{mem_space::ptr::SyscallString, Process},
};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn link(oldpath: SyscallString, newpath: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let oldpath_str = oldpath
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let _old_path = Path::new(oldpath_str)?;

	let newpath_str = newpath
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let _new_path = Path::new(newpath_str)?;

	// TODO Get file at `old_path`
	// TODO Create the link to the file

	Ok(0)
}
