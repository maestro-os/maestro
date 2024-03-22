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

//! The procfs is a virtual filesystem which provides informations about
//! processes.

mod mem_info;
mod proc_dir;
mod self_link;
mod sys_dir;
mod uptime;
mod version;

use super::{
	kernfs,
	kernfs::{node::DummyKernFSNode, KernFS},
	Filesystem, FilesystemType,
};
use crate::{
	file::{
		fs::Statfs,
		path::PathBuf,
		perm::{Gid, Uid},
		DirEntry, File, FileContent, FileType, INode, Mode,
	},
	process,
	process::{oom, pid::Pid},
};
use core::{alloc::AllocError, any::Any};
use mem_info::MemInfo;
use proc_dir::ProcDir;
use self_link::SelfNode;
use sys_dir::SysDir;
use uptime::Uptime;
use utils::{
	boxed::Box,
	collections::{hashmap::HashMap, string::String},
	errno,
	errno::{AllocResult, EResult},
	format,
	io::IO,
	lock::Mutex,
	ptr::arc::Arc,
};
use version::Version;

/// A procfs.
///
/// On the inside, the procfs works using a kernfs.
#[derive(Debug)]
pub struct ProcFS {
	/// The kernfs.
	fs: KernFS,
	/// The list of registered processes with their directory's inode.
	procs: HashMap<Pid, INode>,
}

impl ProcFS {
	/// Creates a new instance.
	///
	/// `readonly` tells whether the filesystem is readonly.
	pub fn new(readonly: bool) -> EResult<Self> {
		let mut fs = Self {
			fs: KernFS::new(b"procfs".try_into()?, readonly)?,
			procs: HashMap::new(),
		};

		let mut entries = HashMap::new();

		// Create /proc/meminfo
		let node = MemInfo {};
		let inode = fs.fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"meminfo".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		// Create /proc/mounts
		let node =
			DummyKernFSNode::new(0o777, 0, 0, FileContent::Link(b"self/mounts".try_into()?));
		let inode = fs.fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"mounts".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Link,
			},
		)?;

		// Create /proc/self
		let node = SelfNode {};
		let inode = fs.fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"self".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Link,
			},
		)?;

		// Create /proc/sys
		let node = SysDir::new(&mut fs.fs)?;
		let inode = fs.fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"sys".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Directory,
			},
		)?;

		// Create /proc/uptime
		let node = Uptime {};
		let inode = fs.fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"uptime".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		// Create /proc/version
		let node = Version {};
		let inode = fs.fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"version".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		// Add the root node
		let root_node = DummyKernFSNode::new(0o555, 0, 0, FileContent::Directory(entries));
		fs.fs.set_root(Box::new(root_node)?)?;

		// Add existing processes
		{
			let mut scheduler = process::get_scheduler().lock();
			for (pid, _) in scheduler.iter_process() {
				fs.add_process(*pid)?;
			}
		}

		Ok(fs)
	}

	/// Adds a process with the given PID `pid` to the filesystem.
	pub fn add_process(&mut self, pid: Pid) -> EResult<()> {
		// Create the process's node
		let proc_node = ProcDir::new(pid, &mut self.fs)?;
		let inode = self.fs.add_node(Box::new(proc_node)?)?;
		oom::wrap(|| self.procs.insert(pid, inode));

		// Insert the process's entry at the root of the filesystem
		let root = self.fs.get_node_mut(kernfs::ROOT_INODE).unwrap();
		oom::wrap(|| {
			let mut content = root.get_content().map_err(|_| AllocError)?;
			let FileContent::Directory(entries) = &mut *content else {
				unreachable!();
			};
			entries.insert(
				format!("{pid}")?,
				DirEntry {
					entry_type: FileType::Directory,
					inode,
				},
			)
		});

		Ok(())
	}

	/// Removes the process with pid `pid` from the filesystem.
	///
	/// If the process doesn't exist, the function does nothing.
	pub fn remove_process(&mut self, pid: Pid) -> AllocResult<()> {
		let Some(inode) = self.procs.remove(&pid) else {
			return Ok(());
		};

		// Remove the process's entry from the root of the filesystem
		let root = self.fs.get_node_mut(kernfs::ROOT_INODE).unwrap();
		oom::wrap(|| {
			let mut content = root.get_content().map_err(|_| AllocError)?;
			let FileContent::Directory(entries) = &mut *content else {
				unreachable!();
			};
			entries.remove(&format!("{pid}")?);
			Ok(())
		});

		// Remove the node
		if let Some(mut node) = oom::wrap(|| self.fs.remove_node(inode).map_err(|_| AllocError)) {
			let node = node.as_mut() as &mut dyn Any;

			if let Some(node) = node.downcast_mut::<ProcDir>() {
				node.drop_inner(&mut self.fs);
			}
		}
		Ok(())
	}
}

impl Filesystem for ProcFS {
	fn get_name(&self) -> &[u8] {
		self.fs.get_name()
	}

	fn is_readonly(&self) -> bool {
		self.fs.is_readonly()
	}

	fn use_cache(&self) -> bool {
		self.fs.use_cache()
	}

	fn get_root_inode(&self) -> INode {
		self.fs.get_root_inode()
	}

	fn get_stat(&self, io: &mut dyn IO) -> EResult<Statfs> {
		self.fs.get_stat(io)
	}

	fn get_inode(
		&mut self,
		io: &mut dyn IO,
		parent: Option<INode>,
		name: &[u8],
	) -> EResult<INode> {
		self.fs.get_inode(io, parent, name)
	}

	fn load_file(&mut self, io: &mut dyn IO, inode: INode, name: String) -> EResult<File> {
		self.fs.load_file(io, inode, name)
	}

	fn add_file(
		&mut self,
		_io: &mut dyn IO,
		_parent_inode: INode,
		_name: String,
		_uid: Uid,
		_gid: Gid,
		_mode: Mode,
		_content: FileContent,
	) -> EResult<File> {
		Err(errno!(EACCES))
	}

	fn add_link(
		&mut self,
		_io: &mut dyn IO,
		_parent_inode: INode,
		_name: &[u8],
		_inode: INode,
	) -> EResult<()> {
		Err(errno!(EACCES))
	}

	fn update_inode(&mut self, _io: &mut dyn IO, _file: &File) -> EResult<()> {
		Ok(())
	}

	fn remove_file(
		&mut self,
		_io: &mut dyn IO,
		_parent_inode: INode,
		_name: &[u8],
	) -> EResult<(u16, INode)> {
		Err(errno!(EACCES))
	}

	fn read_node(
		&mut self,
		io: &mut dyn IO,
		inode: INode,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		self.fs.read_node(io, inode, off, buf)
	}

	fn write_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &[u8]) -> EResult<()> {
		self.fs.write_node(io, inode, off, buf)
	}
}

/// Structure representing the procfs file system type.
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
		_io: &mut dyn IO,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<Mutex<dyn Filesystem>>> {
		Ok(Arc::new(Mutex::new(ProcFS::new(readonly)?))?)
	}
}
