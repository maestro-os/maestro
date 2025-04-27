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

//! `getresgid` returns the real, effective and saved group ID of the current process.

use crate::{
	file::perm::{AccessProfile, Gid, Uid},
	memory::user::UserPtr,
	process::Process,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{errno, errno::EResult, ptr::arc::Arc};

pub fn getresgid(
	Args((rgid, egid, sgid)): Args<(UserPtr<Gid>, UserPtr<Gid>, UserPtr<Gid>)>,
	ap: AccessProfile,
) -> EResult<usize> {
	rgid.copy_to_user(&ap.gid)?;
	egid.copy_to_user(&ap.egid)?;
	sgid.copy_to_user(&ap.sgid)?;
	Ok(0)
}
