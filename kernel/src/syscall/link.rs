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

//! The `link` system call allows to create a hard link.

use super::Args;
use crate::process::{mem_space::copy::SyscallString, Process};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
};

pub fn link(Args((oldpath, newpath)): Args<(SyscallString, SyscallString)>) -> EResult<usize> {
	let oldpath_str = oldpath.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let _old_path = PathBuf::try_from(oldpath_str)?;
	let newpath_str = newpath.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let _new_path = PathBuf::try_from(newpath_str)?;
	// TODO Get file at `old_path`
	// TODO Create the link to the file
	Ok(0)
}
