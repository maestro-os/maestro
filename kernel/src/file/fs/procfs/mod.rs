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

//! The procfs is a virtual filesystem which provides information about
//! processes.

mod mem_info;
mod proc_dir;
mod self_link;
mod sys_dir;
mod uptime;
mod version;

use super::{kernfs::KernFS, Filesystem, FilesystemType, NodeOps};
use crate::{
	file::{
		fs::{
			kernfs::node::{KernFSNode, StaticLink},
			Statfs,
		},
		path::PathBuf,
		DirEntry, File, FileType, INode, Mode,
	},
	process::{pid::Pid, Process},
};
use mem_info::MemInfo;
use self_link::SelfNode;
use sys_dir::SysDir;
use uptime::Uptime;
use utils::{
	boxed::Box,
	collections::hashmap::HashMap,
	errno,
	errno::EResult,
	io::IO,
	lock::Mutex,
	ptr::{arc::Arc, cow::Cow},
};
use version::Version;

/// The root directory of the procfs.
#[derive(Debug)]
struct RootDir;

impl RootDir {
	/// Static entries of the root directory, as opposed to the dynamic ones that represent
	/// processes.
	const STATIC_ENTRIES: &'static [(&'static [u8], &'static dyn KernFSNode)] = &[
		(b"meminfo", &MemInfo {}),
		(b"mounts", &StaticLink::<b"self/mounts"> {}),
		(b"self", &SelfNode {}),
		(b"sys", &SysDir {}),
		(b"uptime", &Uptime {}),
		(b"version", &Version {}),
	];
}

impl KernFSNode for RootDir {
	fn get_file_type(&self) -> FileType {
		FileType::Directory
	}

	fn get_mode(&self) -> Mode {
		0o555
	}
}

impl NodeOps for RootDir {
	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		_buf: &mut [u8],
	) -> EResult<u64> {
		Err(errno!(EISDIR))
	}

	fn write_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		_buf: &[u8],
	) -> EResult<u64> {
		Err(errno!(EISDIR))
	}

	/// This returned offset is junk and should be ignored.
	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, u64)>> {
		let entry = core::str::from_utf8(name)
			.ok()
			// Check for process from pid
			.and_then(|s| {
				let pid: Pid = s.parse().ok()?;
				// Check the process exists
				Process::get_by_pid(pid)?;
				// Return the entry for the process
				Some(DirEntry {
					inode: 0,
					entry_type: FileType::Directory,
					name: Cow::Borrowed(name),
				})
			})
			// Search in static entries
			.or_else(|| {
				let index = Self::STATIC_ENTRIES
					.binary_search_by(|(n, _)| (*n).cmp(name))
					.ok()?;
				let (name, node) = Self::STATIC_ENTRIES[index];
				Some(DirEntry {
					inode: 0,
					entry_type: node.get_file_type(),
					name: Cow::Borrowed(name),
				})
			})
			.map(|entry| (entry, 0));
		Ok(entry)
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let entry = Self::STATIC_ENTRIES
			.get(off)
			.map(|(name, node)| DirEntry {
				inode: 0,
				entry_type: node.get_file_type(),
				name: Cow::Borrowed(*name),
			})
			.or_else(|| {
				// TODO iterate on processes
				todo!()
			});
		Ok(entry.map(|e| (e, (off + 1) as u64)))
	}
}

/// A procfs.
///
/// On the inside, the procfs works using a kernfs.
#[derive(Debug)]
pub struct ProcFS {
	/// The inner kernfs.
	inner: KernFS<true>,
	/// The list of registered processes with their directory's inode.
	procs: HashMap<Pid, INode>,
}

impl ProcFS {
	/// Creates a new instance.
	///
	/// `readonly` tells whether the filesystem is readonly.
	pub fn new() -> EResult<Self> {
		let mut fs = Self {
			inner: KernFS::new()?,
			procs: HashMap::new(),
		};
		fs.inner.set_root(Box::new(RootDir {})?)?;
		Ok(fs)
	}
}

impl Filesystem for ProcFS {
	fn get_name(&self) -> &[u8] {
		b"procfs"
	}

	fn is_readonly(&self) -> bool {
		true
	}

	fn use_cache(&self) -> bool {
		self.inner.use_cache()
	}

	fn get_root_inode(&self) -> INode {
		self.inner.get_root_inode()
	}

	fn get_stat(&self) -> EResult<Statfs> {
		self.inner.get_stat()
	}

	fn load_file(&self, inode: INode) -> EResult<File> {
		self.inner.load_file(inode)
	}

	fn add_file(&self, _parent_inode: INode, _name: &[u8], _node: File) -> EResult<File> {
		Err(errno!(EACCES))
	}

	fn add_link(&self, _parent_inode: INode, _name: &[u8], _inode: INode) -> EResult<()> {
		Err(errno!(EACCES))
	}

	fn update_inode(&self, _file: &File) -> EResult<()> {
		Ok(())
	}

	fn remove_file(&self, _parent_inode: INode, _name: &[u8]) -> EResult<(u16, INode)> {
		Err(errno!(EACCES))
	}
}

/// The procfs filesystem type.
pub struct ProcFsType {}

impl FilesystemType for ProcFsType {
	fn get_name(&self) -> &'static [u8] {
		b"procfs"
	}

	fn detect(&self, _io: &mut dyn IO) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_io: Option<Arc<Mutex<dyn IO>>>,
		_mountpath: PathBuf,
		_readonly: bool,
	) -> EResult<Arc<dyn Filesystem>> {
		Ok(Arc::new(ProcFS::new()?)?)
	}
}
