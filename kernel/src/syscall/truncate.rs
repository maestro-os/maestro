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

//! The `truncate` syscall allows to truncate a file.

use crate::{
	file::{vfs, vfs::ResolutionSettings, File, O_WRONLY},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
};

pub fn truncate(Args((path, length)): Args<(SyscallString, usize)>) -> EResult<usize> {
	let proc = Process::current();
	let rs = ResolutionSettings::for_process(&proc, true);
	let path = path.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let ent = vfs::get_file_from_path(&path, &rs)?;
	// Permission check
	if !rs.access_profile.can_write_file(&ent.stat()) {
		return Err(errno!(EACCES));
	}
	// Truncate
	let file = File::open_entry(ent, O_WRONLY)?;
	file.ops.truncate(&file, length as _)?;
	Ok(0)
}
