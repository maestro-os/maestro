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

//! The `dup2` syscall allows to duplicate a file descriptor, specifying the id
//! of the newly created file descriptor.

use crate::{file::fd::NewFDConstraint, process::Process, syscall::Args};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn dup2(Args((oldfd, newfd)): Args<(c_int, c_int)>) -> EResult<usize> {
	if oldfd < 0 || newfd < 0 {
		return Err(errno!(EBADF));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let fds_mutex = proc.file_descriptors.as_ref().unwrap();
	let mut fds = fds_mutex.lock();

	let (newfd_id, _) = fds.duplicate_fd(oldfd as _, NewFDConstraint::Fixed(newfd as _), false)?;
	Ok(newfd_id as _)
}
