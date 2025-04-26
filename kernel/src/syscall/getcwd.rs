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

//! The `getcwd` system call allows to retrieve the current working directory of
//! the current process.

use crate::{file::vfs, memory::user::UserSlice, process::Process, syscall::Args};
use core::intrinsics::unlikely;
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn getcwd(Args((buf, size)): Args<(*mut u8, usize)>, proc: Arc<Process>) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, size)?;
	let cwd = vfs::Entry::get_path(&proc.fs.lock().cwd)?;
	if unlikely(size < cwd.len() + 1) {
		return Err(errno!(ERANGE));
	}
	buf.copy_to_user(0, cwd.as_bytes())?;
	buf.copy_to_user(cwd.len(), b"\0")?;
	Ok(buf.as_ptr() as _)
}
