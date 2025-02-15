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
	file::{vfs, vfs::ResolutionSettings, FileType},
	process::{
		mem_space::copy::{SyscallSlice, SyscallString},
		Process,
	},
	syscall::Args,
};
use utils::{
	collections::{path::PathBuf, vec::Vec},
	errno,
	errno::{EResult, Errno},
	vec,
};

pub fn readlink(
	Args((pathname, buf, bufsiz)): Args<(SyscallString, SyscallSlice<u8>, usize)>,
) -> EResult<usize> {
	// process lock has to be dropped to avoid deadlock with procfs
	let (path, rs) = {
		let proc = Process::current();

		// Get file's path
		let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let rs = ResolutionSettings::for_process(&proc, false);
		(path, rs)
	};
	let ent = vfs::get_file_from_path(&path, &rs)?;
	// Validation
	if ent.get_type()? != FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Read link
	let mut buffer = vec![0; bufsiz]?;
	let len = ent.node().node_ops.readlink(&ent.node(), &mut buffer)?;
	buf.copy_to_user(0, &buffer)?;
	Ok(len as _)
}
