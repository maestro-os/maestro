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

//! The `osrelease` node returns the current release of the kernel.

use crate::{
	file::{
		fs::{kernfs::node::KernFSNode, Filesystem, NodeOps},
		perm::{Gid, Uid},
		DirEntry, FileType, INode, Mode,
	},
	format_content,
};
use utils::{errno, errno::EResult};

/// The `osrelease` file.
#[derive(Debug)]
pub struct OsRelease {}

impl KernFSNode for OsRelease {
	fn get_file_type(&self) -> FileType {
		FileType::Regular
	}

	fn get_mode(&self) -> Mode {
		0o444
	}

	fn get_uid(&self) -> Uid {
		0
	}

	fn get_gid(&self) -> Gid {
		0
	}
}

impl NodeOps for OsRelease {
	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		format_content!(off, buf, "{}\n", crate::VERSION)
	}

	fn write_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		_buf: &[u8],
	) -> EResult<u64> {
		Err(errno!(EACCES))
	}

	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_name: &'n [u8],
	) -> EResult<Option<DirEntry<'n>>> {
		Err(errno!(ENOTDIR))
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		Err(errno!(ENOTDIR))
	}
}
