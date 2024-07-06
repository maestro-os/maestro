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

//! The pipe2 system call allows to create a pipe with given flags.

use crate::{
	file::{buffer, buffer::pipe::PipeBuffer, open_file, open_file::OpenFile, vfs},
	process::Process,
	syscall::{Args, SyscallPtr},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
	TryDefault,
};

pub fn pipe2(Args((pipefd, flags)): Args<(SyscallPtr<[c_int; 2]>, c_int)>) -> EResult<usize> {
	let accepted_flags = open_file::O_CLOEXEC | open_file::O_DIRECT | open_file::O_NONBLOCK;
	if flags & !accepted_flags != 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let (mem_space, fds_mutex) = {
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap().clone();
		let fds_mutex = proc.file_descriptors.clone().unwrap();
		(mem_space, fds_mutex)
	};

	let loc = buffer::register(None, Arc::new(Mutex::new(PipeBuffer::try_default()?))?)?;
	let file = vfs::get_file_from_location(loc)?;

	let open_file0 = OpenFile::new(file.clone(), None, open_file::O_RDONLY)?;
	let open_file1 = OpenFile::new(file, None, open_file::O_WRONLY)?;

	let mut fds = fds_mutex.lock();
	let (fd0_id, fd1_id) = fds.create_fd_pair(open_file0, open_file1)?;

	let mut mem_space_guard = mem_space.lock();
	let pipefd_slice = pipefd
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;
	pipefd_slice[0] = fd0_id as _;
	pipefd_slice[1] = fd1_id as _;

	Ok(0)
}
