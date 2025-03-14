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

use crate::{
	device::BlkDev,
	file::{
		fs::{
			downcast_fs, kernfs, kernfs::NodeStorage, FileOps, Filesystem, FilesystemOps,
			FilesystemType, NodeOps, StatSet, Statfs,
		},
		perm::{ROOT_GID, ROOT_UID},
		vfs,
		vfs::node::Node,
		DirContext, DirEntry, File, FileType, Stat,
	},
	sync::mutex::Mutex,
};
use core::{
	any::Any,
	cmp::{max, min},
	intrinsics::unlikely,
};
use utils::{
	boxed::Box,
	collections::{hashmap::HashMap, path::PathBuf, vec::Vec},
	errno,
	errno::EResult,
	limits::{NAME_MAX, PAGE_SIZE},
	ptr::{arc::Arc, cow::Cow},
	TryClone,
};
// TODO count memory usage to enforce quota

// TODO use rwlock
/// The content of a [`TmpFSNode`]
#[derive(Debug)]
enum NodeContent {
	/// Regular file content
	Regular(Mutex<Vec<u8>>),
	/// Directory entries
	Directory(Mutex<HashMap<Cow<'static, [u8]>, Arc<Node>>>),
	/// Symbolic link path
	Link(Vec<u8>),
	/// No content
	None,
}

impl NodeContent {
	/// Returns a reference to the content from the given [`NodeOps`].
	fn from_ops(ops: &dyn NodeOps) -> &Self {
		(ops as &dyn Any).downcast_ref().unwrap()
	}
}

impl NodeOps for NodeContent {
	fn set_stat(&self, _node: &Node, _set: &StatSet) -> EResult<()> {
		Ok(())
	}

	fn lookup_entry(&self, _dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let NodeContent::Directory(entries) = self else {
			return Err(errno!(ENOTDIR));
		};
		ent.node = entries.lock().get(ent.name.as_ref()).cloned();
		Ok(())
	}

	fn iter_entries(&self, _dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let NodeContent::Directory(entries) = self else {
			return Err(errno!(ENOTDIR));
		};
		let off: usize = ctx.off.try_into().map_err(|_| errno!(EOVERFLOW))?;
		let entries = entries.lock();
		let iter = entries.iter().skip(off);
		for (name, node) in iter {
			let ent = DirEntry {
				inode: node.inode,
				entry_type: node.stat.lock().get_type(),
				name,
			};
			if !(*ctx.write)(&ent)? {
				break;
			}
			ctx.off += 1;
		}
		Ok(())
	}

	fn link(&self, parent: Arc<Node>, ent: &vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		// Check if an entry already exists
		let NodeContent::Directory(parent_entries) = self else {
			return Err(errno!(ENOTDIR));
		};
		let mut parent_entries = parent_entries.lock();
		if parent_entries.get(ent.name.as_ref()).is_some() {
			return Err(errno!(EEXIST));
		}
		// Insert entry
		let node = ent.node();
		parent_entries.insert(Cow::Owned(ent.name.try_clone()?), node.clone())?;
		// If this is a directory, create `.` and `..`
		let content = NodeContent::from_ops(&*node.node_ops);
		if let NodeContent::Directory(ents) = content {
			let mut ents = ents.lock();
			ents.insert(Cow::Borrowed(b"."), node.clone())?;
			ents.insert(Cow::Borrowed(b".."), parent.clone())?;
		}
		node.stat.lock().nlink += 1;
		Ok(())
	}

	fn unlink(&self, parent: &Node, ent: &vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		// Find entry
		let NodeContent::Directory(parent_entries) = self else {
			return Err(errno!(ENOTDIR));
		};
		let mut parent_entries = parent_entries.lock();
		let node = parent_entries
			.get(ent.name.as_ref())
			.ok_or_else(|| errno!(ENOENT))?;
		// Handle directory-specifics
		let content = NodeContent::from_ops(&*node.node_ops);
		if let NodeContent::Directory(ents) = content {
			// If not empty, error
			let mut ents = ents.lock();
			if ents.len() > 2
				|| ents
					.iter()
					.any(|(name, _)| name.as_ref() != b"." && name.as_ref() != b"..")
			{
				return Err(errno!(ENOTEMPTY));
			}
			// Remove `.` and `..` to break cycles
			ents.clear();
			// Decrement the number of hard links to the parent (because of the entry `..` in the
			// removed node)
			parent.stat.lock().nlink -= 1;
		}
		// Remove
		node.stat.lock().nlink -= 1;
		parent_entries.remove(ent.name.as_ref());
		Ok(())
	}

	fn readlink(&self, _node: &Node, buf: &mut [u8]) -> EResult<usize> {
		let NodeContent::Regular(content) = self else {
			return Err(errno!(EINVAL));
		};
		let content = content.lock();
		let len = min(buf.len(), content.len());
		buf[..len].copy_from_slice(&content[..len]);
		Ok(len)
	}

	fn writelink(&self, node: &Node, buf: &[u8]) -> EResult<()> {
		let NodeContent::Regular(content) = self else {
			return Err(errno!(EINVAL));
		};
		let mut content = content.lock();
		content.resize(buf.len(), 0)?;
		content.copy_from_slice(buf);
		// Update status
		node.stat.lock().size = buf.len() as _;
		Ok(())
	}

	fn rename(
		&self,
		_old_parent: &Node,
		_old_name: &vfs::Entry,
		_new_parent: &Node,
		_new_name: &vfs::Entry,
	) -> EResult<()> {
		todo!()
	}
}

/// Open file operations.
#[derive(Debug)]
pub struct TmpFSFile;

impl FileOps for TmpFSFile {
	fn read(&self, file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let node_ops = &*file.node().unwrap().node_ops;
		let content = NodeContent::from_ops(node_ops);
		let NodeContent::Regular(content) = content else {
			return Err(errno!(EINVAL));
		};
		let content = content.lock();
		if off > content.len() as u64 {
			return Err(errno!(EINVAL));
		}
		let off = off as usize;
		let len = min(buf.len(), content.len() - off);
		buf[..len].copy_from_slice(&content[off..(off + len)]);
		Ok(len)
	}

	fn write(&self, file: &File, off: u64, buf: &[u8]) -> EResult<usize> {
		let node_ops = &*file.node().unwrap().node_ops;
		let content = NodeContent::from_ops(node_ops);
		let NodeContent::Regular(content) = content else {
			return Err(errno!(EINVAL));
		};
		let mut content = content.lock();
		if off > content.len() as u64 {
			return Err(errno!(EINVAL));
		}
		let off = off as usize;
		let Some(end) = off.checked_add(buf.len()) else {
			return Err(errno!(EOVERFLOW));
		};
		let new_len = max(content.len(), end);
		content.resize(new_len, 0)?;
		content[off..end].copy_from_slice(buf);
		// Update status
		let node = file.node().unwrap();
		node.stat.lock().size = new_len as _;
		Ok(buf.len())
	}

	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let node_ops = &*file.node().unwrap().node_ops;
		let content = NodeContent::from_ops(node_ops);
		let NodeContent::Regular(content) = content else {
			return Err(errno!(EINVAL));
		};
		content.lock().truncate(size as _);
		Ok(())
	}
}

/// A temporary file system.
///
/// On the inside, the tmpfs works using a kernfs.
#[derive(Debug)]
pub struct TmpFS {
	/// Tells whether the filesystem is readonly.
	readonly: bool,
	/// The inner kernfs.
	nodes: Mutex<NodeStorage>,
}

impl FilesystemOps for TmpFS {
	fn get_name(&self) -> &[u8] {
		b"tmpfs"
	}

	fn get_stat(&self) -> EResult<Statfs> {
		Ok(Statfs {
			f_type: 0,
			f_bsize: PAGE_SIZE as _,
			f_blocks: 0,
			f_bfree: 0,
			f_bavail: 0,
			f_files: 0,
			f_ffree: 0,
			f_fsid: Default::default(),
			f_namelen: NAME_MAX as _,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn root(&self, _fs: Arc<Filesystem>) -> EResult<Arc<Node>> {
		self.nodes.lock().get_node(kernfs::ROOT_INODE).cloned()
	}

	fn create_node(&self, fs: Arc<Filesystem>, stat: Stat) -> EResult<Arc<Node>> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		// Prepare content
		let file_type = stat.get_type().ok_or_else(|| errno!(EINVAL))?;
		let content = match file_type {
			FileType::Regular => NodeContent::Regular(Default::default()),
			FileType::Directory => NodeContent::Directory(Default::default()),
			FileType::Link => NodeContent::Link(Default::default()),
			_ => NodeContent::None,
		};
		// Insert node
		let mut nodes = self.nodes.lock();
		let (inode, slot) = nodes.get_free_slot()?;
		let node = Arc::new(Node {
			inode,
			fs,

			stat: Mutex::new(stat),

			node_ops: Box::new(content)?,
			file_ops: Box::new(TmpFSFile)?,

			lock: Default::default(),
			cache: Default::default(),
		})?;
		*slot = Some(node.clone());
		Ok(node)
	}

	fn destroy_node(&self, node: &Node) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		self.nodes.lock().remove_node(node.inode);
		Ok(())
	}
}

/// The tmpfs filesystem type.
pub struct TmpFsType;

impl FilesystemType for TmpFsType {
	fn get_name(&self) -> &'static [u8] {
		b"tmpfs"
	}

	fn detect(&self, _dev: &BlkDev) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_dev: Option<Arc<BlkDev>>,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<Filesystem>> {
		let fs = Filesystem::new(
			0,
			Box::new(TmpFS {
				readonly,
				nodes: Mutex::new(NodeStorage::new()?),
			})?,
		)?;
		let root = Arc::new(Node {
			inode: 0,
			fs: fs.clone(),

			stat: Mutex::new(Stat {
				mode: FileType::Directory.to_mode() | 0o1777,
				nlink: 2, // `.` and `..`
				uid: ROOT_UID,
				gid: ROOT_GID,
				size: 0,
				blocks: 0,
				dev_major: 0,
				dev_minor: 0,
				ctime: 0,
				mtime: 0,
				atime: 0,
			}),

			node_ops: Box::new(NodeContent::Directory(Default::default()))?,
			file_ops: Box::new(TmpFSFile)?,

			lock: Default::default(),
			cache: Default::default(),
		})?;
		// Insert node
		downcast_fs::<TmpFS>(&*fs.ops)
			.nodes
			.lock()
			.set_root(root.clone())?;
		// Insert `.` and `..`
		let content = NodeContent::from_ops(&*root.node_ops);
		let NodeContent::Directory(entries) = content else {
			unreachable!();
		};
		let mut entries = entries.lock();
		entries.insert(Cow::Borrowed(b"."), root.clone())?;
		entries.insert(Cow::Borrowed(b".."), root.clone())?;
		Ok(fs)
	}
}
