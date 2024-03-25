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

//! The pipe system call allows to create a pipe.

use crate::{
	file::{buffer, buffer::pipe::PipeBuffer, open_file, open_file::OpenFile, vfs},
	process::{mem_space::ptr::SyscallPtr, Process},
};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno, lock::Mutex, ptr::arc::Arc, TryDefault};

#[syscall]
pub fn pipe(pipefd: SyscallPtr<[c_int; 2]>) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let (mem_space, fds_mutex) = {
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap().clone();
		let fds_mutex = proc.file_descriptors.clone().unwrap();
		(mem_space, fds_mutex)
	};

	let loc = buffer::register(None, Arc::new(Mutex::new(PipeBuffer::try_default()?))?)?;
	let file = vfs::get_file_from_location(&loc)?;

	let open_file0 = OpenFile::new(file.clone(), open_file::O_RDONLY)?;
	let open_file1 = OpenFile::new(file, open_file::O_WRONLY)?;

	let mut fds = fds_mutex.lock();
	let mut mem_space_guard = mem_space.lock();

	let pipefd_slice = pipefd
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;
	let fd0 = fds.create_fd(0, open_file0)?;
	pipefd_slice[0] = fd0.get_id() as _;
	let fd1 = fds.create_fd(0, open_file1)?;
	pipefd_slice[1] = fd1.get_id() as _;

	Ok(0)
}
