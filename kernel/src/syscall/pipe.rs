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
	file,
	file::{File, fd::FileDescriptorTable, pipe::PipeBuffer},
	memory::user::UserPtr,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{boxed::Box, errno::EResult, ptr::arc::Arc};

pub fn pipe(
	Args(pipefd): Args<UserPtr<[c_int; 2]>>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let ops = Arc::new(PipeBuffer::new()?)?;
	let file0 = File::open_floating(ops.clone(), file::O_RDONLY)?;
	let file1 = File::open_floating(ops, file::O_WRONLY)?;
	let (fd0_id, fd1_id) = fds.lock().create_fd_pair(file0, file1)?;
	pipefd.copy_to_user(&[fd0_id as _, fd1_id as _])?;
	Ok(0)
}
