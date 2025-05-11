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

//! The `chroot` system call allows to virtually redefine the system's root for
//! the current process.

use crate::{
	file::{
		vfs,
		vfs::{mountpoint, ResolutionSettings},
		FileType,
	},
	memory::user::UserString,
	process::Process,
	syscall::Args,
};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn chroot(
	Args(path): Args<UserString>,
	proc: Arc<Process>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	// Check permission
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	let path = path.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let rs = ResolutionSettings {
		root: vfs::ROOT.clone(),
		..rs
	};
	// Get file
	let ent = vfs::get_file_from_path(&path, &rs)?;
	if ent.get_type()? != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	proc.fs.lock().chroot = ent;
	Ok(0)
}
