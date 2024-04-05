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

//! The `version` file returns the version of the kernel.

use crate::file::{
	fs::{
		kernfs::node::{content_chunks, KernFSNode},
		Filesystem, NodeOps,
	},
	DirEntry, FileType, INode, Mode,
};
use utils::{errno, errno::EResult};

/// Kernel version file.
#[derive(Debug)]
pub struct Version;

impl KernFSNode for Version {
	fn get_file_type(&self) -> FileType {
		FileType::Regular
	}

	fn get_mode(&self) -> Mode {
		0o444
	}
}

impl NodeOps for Version {
	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		let iter = [crate::NAME, " version ", crate::VERSION]
			.into_iter()
			.map(|s| Ok(s.as_bytes()));
		content_chunks(off, buf, iter)
	}

	fn write_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		_buf: &[u8],
	) -> EResult<()> {
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
