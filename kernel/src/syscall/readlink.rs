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

//! The `readlink` syscall allows to read the target of a symbolic link.

use crate::{
	file::{path::PathBuf, vfs, vfs::ResolutionSettings, FileType},
	process::{
		mem_space::ptr::{SyscallSlice, SyscallString},
		Process,
	},
};
use macros::syscall;
use utils::{errno, errno::Errno, io::IO};

#[syscall]
pub fn readlink(
	pathname: SyscallString,
	buf: SyscallSlice<u8>,
	bufsiz: usize,
) -> Result<i32, Errno> {
	// process lock has to be dropped to avoid deadlock with procfs
	let (mem_space_mutex, path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space_mutex = proc.get_mem_space().unwrap().clone();
		let mem_space = mem_space_mutex.lock();

		// Get file's path
		let path = pathname.get(&mem_space)?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		drop(mem_space);

		let rs = ResolutionSettings::for_process(&proc, false);
		(mem_space_mutex, path, rs)
	};
	let file_mutex = vfs::get_file_from_path(&path, &rs)?;
	let mut file = file_mutex.lock();
	if file.get_type() != FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Copy to userspace buffer
	let mut mem_space = mem_space_mutex.lock();
	let buffer = buf.get_mut(&mut mem_space, bufsiz)?.ok_or(errno!(EFAULT))?;
	let (len, _) = file.read(0, buffer)?;
	Ok(len as _)
}
