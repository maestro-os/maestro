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

//! Implementation of the `maps` node which returns the list of memory mappings.

use crate::{
	file::{File, fs::FileOps, vfs},
	format_content,
	memory::user::UserSlice,
	process::{
		Process,
		mem_space::{MAP_SHARED, MemSpace, PROT_EXEC, PROT_READ, PROT_WRITE, mapping::MemMapping},
		pid::Pid,
	},
};
use core::{fmt, fmt::Formatter};
use utils::{collections::path::PathBuf, errno, errno::EResult, limits::PAGE_SIZE};

/// The `maps` node.
#[derive(Debug)]
pub struct Maps(pub Pid);

impl FileOps for Maps {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let Some(mem_space) = proc.mem_space_opt() else {
			return Ok(0);
		};
		format_content!(off, buf, "{}", MapsDisplay(mem_space))
	}
}

struct MapsDisplay<'m>(&'m MemSpace);

impl fmt::Display for MapsDisplay<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		self.0.mappings(|mappings| {
			for (begin, mapping) in mappings.iter() {
				let end = *begin + mapping.size.get() * PAGE_SIZE;
				let perms = Perms(mapping);
				let vfs_entry = mapping.file.as_ref().and_then(|f| f.vfs_entry.as_ref());
				let (major, minor, inode, pathname) = match vfs_entry {
					Some(vfs_entry) => {
						let node = vfs_entry
							.node
							.as_ref()
							// cannot be a negative entry
							.unwrap();
						let stat = node.stat();
						// TODO figure how to handle memory allocation failures
						let path = vfs::Entry::get_path(vfs_entry).unwrap_or(PathBuf::empty());
						(stat.dev_major, stat.dev_minor, node.inode, path)
					}
					None => (0, 0, 0, PathBuf::empty()),
				};
				writeln!(
					f,
					"{begin:x}-{end:x} {perms} {off} {major}:{minor} {inode:<25} {pathname}",
					begin = begin.0,
					end = end.0,
					off = mapping.off
				)?;
			}
			Ok(())
		})
	}
}

struct Perms<'m>(&'m MemMapping);

impl fmt::Display for Perms<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let r = if self.0.prot & PROT_READ != 0 {
			'r'
		} else {
			'-'
		};
		let w = if self.0.prot & PROT_WRITE != 0 {
			'w'
		} else {
			'-'
		};
		let e = if self.0.prot & PROT_EXEC != 0 {
			'x'
		} else {
			'-'
		};
		let s = if self.0.flags & MAP_SHARED != 0 {
			's'
		} else {
			'p'
		};
		write!(f, "{r}{w}{e}{s}")
	}
}
