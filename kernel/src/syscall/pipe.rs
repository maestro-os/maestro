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

//! The `pipe` system call allows to create a pipe.

use crate::{
	file::{
		File, FileType, O_CLOEXEC, O_DIRECT, O_NONBLOCK, O_RDONLY, O_WRONLY, fs::float,
		pipe::PipeBuffer,
	},
	memory::user::UserPtr,
	process::Process,
};
use core::{ffi::c_int, hint::unlikely};
use utils::{errno, errno::EResult};

pub fn pipe(pipefd: UserPtr<[c_int; 2]>) -> EResult<usize> {
	let pipe = float::get_entry(PipeBuffer::new()?, FileType::Fifo)?;
	let file0 = File::open_floating(pipe.clone(), O_RDONLY)?;
	let file1 = File::open_floating(pipe, O_WRONLY)?;
	let (fd0_id, fd1_id) = Process::current()
		.file_descriptors()
		.lock()
		.create_fd_pair(file0, file1)?;
	pipefd.copy_to_user(&[fd0_id as _, fd1_id as _])?;
	Ok(0)
}

pub fn pipe2(pipefd: UserPtr<[c_int; 2]>, flags: c_int) -> EResult<usize> {
	// Validation
	if unlikely(flags & !(O_CLOEXEC | O_DIRECT | O_NONBLOCK) != 0) {
		return Err(errno!(EINVAL));
	}
	let pipe = float::get_entry(PipeBuffer::new()?, FileType::Fifo)?;
	let file0 = File::open_floating(pipe.clone(), flags | O_RDONLY)?;
	let file1 = File::open_floating(pipe, flags | O_WRONLY)?;
	let (fd0_id, fd1_id) = Process::current()
		.file_descriptors()
		.lock()
		.create_fd_pair(file0, file1)?;
	pipefd.copy_to_user(&[fd0_id as _, fd1_id as _])?;
	Ok(0)
}
