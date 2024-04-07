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

use super::{
	kernfs::{node::KernFSNode, KernFS},
	Filesystem, FilesystemType,
};
use crate::file::{
	fs::{kernfs::node::DefaultNode, Statfs},
	path::PathBuf,
	File, FileType, INode,
};
use core::{intrinsics::unlikely, mem::size_of};
use utils::{boxed::Box, errno, errno::EResult, io::IO, lock::Mutex, ptr::arc::Arc};

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
	/// Tells whether the filesystem is readonly.
	readonly: bool,
	/// The inner kernfs.
	inner: KernFS<false>,
}

impl TmpFS {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `max_size` is the maximum amount of memory the filesystem can use in bytes.
	/// - `readonly` tells whether the filesystem is readonly.
	pub fn new(max_size: usize, readonly: bool) -> EResult<Self> {
		let root = DefaultNode::new(0, 0, FileType::Directory, 0o777);
		let size = get_used_size(&root);
		let fs = Self {
			max_size,
			size,
			readonly,
			inner: KernFS::<false>::new(Box::new(root)?)?,
		};
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
		b"tmpfs"
	}

	fn is_readonly(&self) -> bool {
		self.readonly
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

	fn add_file(&self, parent_inode: INode, name: &[u8], node: File) -> EResult<File> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		// TODO Update fs's size
		self.inner.add_file(parent_inode, name, node)
	}

	fn add_link(&self, parent_inode: INode, name: &[u8], inode: INode) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		// TODO Update fs's size
		self.inner.add_link(parent_inode, name, inode)
	}

	fn update_inode(&self, file: &File) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		// TODO Update fs's size
		self.inner.update_inode(file)
	}

	fn remove_file(&self, parent_inode: INode, name: &[u8]) -> EResult<(u16, INode)> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		// TODO Update fs's size
		self.inner.remove_file(parent_inode, name)
	}
}

/// The tmpfs filesystem type.
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
		_io: Option<Arc<Mutex<dyn IO>>>,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<dyn Filesystem>> {
		Ok(Arc::new(TmpFS::new(DEFAULT_MAX_SIZE, readonly)?)?)
	}
}
