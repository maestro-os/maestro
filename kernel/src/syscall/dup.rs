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

//! The `dup` syscall allows to duplicate a file descriptor.

use crate::{
	file::fd::{FileDescriptorTable, NewFDConstraint},
	process::Process,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn dup(Args(oldfd): Args<c_int>, fds: Arc<Mutex<FileDescriptorTable>>) -> EResult<usize> {
	let (newfd_id, _) = fds
		.lock()
		.duplicate_fd(oldfd as _, NewFDConstraint::None, false)?;
	Ok(newfd_id as _)
}
