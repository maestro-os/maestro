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

//! Tmpfs (Temporary file system) is, as its name states a temporary filesystem.
//!
//! The files are stored on the kernel's memory and thus are removed when the
//! filesystem is unmounted.

mod node;

use super::{
	kernfs::{node::KernFSNode, KernFS},
	Filesystem, FilesystemType,
};
use crate::file::{
	fs::{kernfs::node::DummyKernFSNode, Statfs},
	path::PathBuf,
	perm::{Gid, Uid},
	File, INode, Mode,
};
use core::mem::size_of;
use node::TmpFSRegular;
use utils::{
	boxed::Box,
	collections::{hashmap::HashMap, string::String},
	errno,
	errno::EResult,
	io::IO,
	lock::Mutex,
	ptr::arc::Arc,
};

/// The default maximum amount of memory the filesystem can use in bytes.
const DEFAULT_MAX_SIZE: usize = 512 * 1024 * 1024;

/// Returns the size in bytes used by the given node `node`.
fn get_used_size<N: KernFSNode>(node: &N) -> usize {
	size_of::<N>() + node.get_size() as usize
}

/// A temporary file system.
///
/// On the inside, the tmpfs works using a kernfs.
#[derive(Debug)]
pub struct TmpFS {
	/// The maximum amount of memory in bytes the filesystem can use.
	max_size: usize,
	/// The currently used amount of memory in bytes.
	size: usize,

	/// The kernfs.
	fs: KernFS,
}

impl TmpFS {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `max_size` is the maximum amount of memory the filesystem can use in bytes.
	/// - `readonly` tells whether the filesystem is readonly.
	pub fn new(max_size: usize, readonly: bool) -> EResult<Self> {
		let mut fs = Self {
			max_size,
			size: 0,

			fs: KernFS::new(b"tmpfs".try_into()?, readonly)?,
		};

		// Adding the root node
		let root_node = DummyKernFSNode::new(0o777, 0, 0, FileContent::Directory(HashMap::new()));
		fs.update_size(get_used_size(&root_node) as _, |fs| {
			fs.fs.set_root(Box::new(root_node)?)?;
			Ok(())
		})?;

		Ok(fs)
	}

	/// Executes the given function `f`.
	///
	/// On success, the function adds `s` to the total size of the filesystem.
	///
	/// If `f` fails, the function doesn't change the total size and returns the
	/// error.
	///
	/// If the new total size is too large, `f` is not executed and the
	/// function returns an error.
	fn update_size<F: FnOnce(&mut Self) -> EResult<()>>(&mut self, s: isize, f: F) -> EResult<()> {
		if s < 0 {
			f(self)?;

			if self.size < (-s as usize) {
				// If the result would underflow, set the total to zero
				self.size = 0;
			} else {
				self.size -= -s as usize;
			}

			Ok(())
		} else if self.size + (s as usize) < self.max_size {
			f(self)?;

			self.size += s as usize;
			Ok(())
		} else {
			Err(errno!(ENOSPC))
		}
	}
}

impl Filesystem for TmpFS {
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
		io: &mut dyn IO,
		parent_inode: INode,
		name: String,
		uid: Uid,
		gid: Gid,
		mode: Mode,
		content: FileContent,
	) -> EResult<File> {
		// TODO Update fs's size

		match content {
			FileContent::Regular => {
				let node = TmpFSRegular::new(mode, uid, gid);
				self.fs.add_file_inner(parent_inode, node, name)
			}

			_ => self
				.fs
				.add_node(io, parent_inode, name, uid, gid, mode, content),
		}
	}

	fn add_link(
		&mut self,
		io: &mut dyn IO,
		parent_inode: INode,
		name: &[u8],
		inode: INode,
	) -> EResult<()> {
		// TODO Update fs's size
		self.fs.add_link(io, parent_inode, name, inode)
	}

	fn update_inode(&mut self, io: &mut dyn IO, file: &File) -> EResult<()> {
		// TODO Update fs's size
		self.fs.update_inode(io, file)
	}

	fn remove_file(
		&mut self,
		io: &mut dyn IO,
		parent_inode: INode,
		name: &[u8],
	) -> EResult<(u16, INode)> {
		// TODO Update fs's size
		self.fs.remove_file(io, parent_inode, name)
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
		// TODO Update fs's size
		self.fs.write_node(io, inode, off, buf)
	}
}

/// Structure representing the tmpfs file system type.
pub struct TmpFsType {}

impl FilesystemType for TmpFsType {
	fn get_name(&self) -> &'static [u8] {
		b"tmpfs"
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
		Ok(Arc::new(Mutex::new(TmpFS::new(
			DEFAULT_MAX_SIZE,
			readonly,
		)?))?)
	}
}
