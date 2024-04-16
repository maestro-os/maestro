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

//! Implementation of the directory of a process in the procfs.

mod cmdline;
mod cwd;
mod exe;
mod mounts;
mod stat;
mod status;

use crate::{
	file::{
		fs::{procfs::get_proc_owner, Filesystem, NodeOps},
		DirEntry, FileType, INode, Stat,
	},
	process::pid::Pid,
};
use cmdline::Cmdline;
use cwd::Cwd;
use exe::Exe;
use mounts::Mounts;
use stat::StatNode;
use status::Status;
use utils::{
	boxed::Box,
	errno,
	errno::{AllocResult, EResult},
	ptr::cow::Cow,
};

/// The directory of a process.
#[derive(Debug)]
pub struct ProcDir(pub Pid);

impl ProcDir {
	/// The list of entries with their respective initializers.
	const ENTRY_INIT: &'static [(
		&'static [u8],
		FileType,
		fn(Pid) -> AllocResult<Box<dyn NodeOps>>,
	)] = &[
		(b"cmdline", FileType::Regular, Self::entry_init::<Cmdline>),
		(b"cwd", FileType::Regular, Self::entry_init::<Cwd>),
		(b"exe", FileType::Regular, Self::entry_init::<Exe>),
		(b"mounts", FileType::Regular, Self::entry_init::<Mounts>),
		(b"stat", FileType::Regular, Self::entry_init::<StatNode>),
		(b"status", FileType::Regular, Self::entry_init::<Status>),
	];

	/// Initialization function for an entry handle.
	fn entry_init<'e, E: 'e + NodeOps + From<Pid>>(
		pid: Pid,
	) -> AllocResult<Box<dyn 'e + NodeOps>> {
		Ok(Box::new(E::from(pid))? as _)
	}
}

impl NodeOps for ProcDir {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		let (uid, gid) = get_proc_owner(self.0);
		Ok(Stat {
			file_type: FileType::Directory,
			mode: 0o555,
			uid,
			gid,
			..Default::default()
		})
	}

	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, u64, Box<dyn NodeOps>)>> {
		let index = Self::ENTRY_INIT
			.binary_search_by(|(n, ..)| (*n).cmp(name))
			.map_err(|_| errno!(ENOENT))?;
		let e = &Self::ENTRY_INIT[index];
		Ok(Some((
			DirEntry {
				inode: 0,
				entry_type: e.1,
				name: Cow::Borrowed(name),
			},
			index as _,
			e.2(self.0)?,
		)))
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let Some((name, entry_type, _)) = &Self::ENTRY_INIT.get(off) else {
			return Ok(None);
		};
		Ok(Some((
			DirEntry {
				inode: 0,
				entry_type: *entry_type,
				name: Cow::Borrowed(name),
			},
			off as u64 + 1,
		)))
	}
}
