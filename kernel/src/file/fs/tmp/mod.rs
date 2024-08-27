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
	device::DeviceIO,
	file::{
		fs::{
			downcast_fs, kernfs, kernfs::NodeStorage, Filesystem, FilesystemType, NodeOps,
			StatSet, Statfs,
		},
		path::PathBuf,
		perm::{Gid, Uid, ROOT_GID, ROOT_UID},
		DirEntry, FileLocation, FileType, INode, Mode, Stat,
	},
	memory,
	time::unit::Timestamp,
};
use core::{
	cmp::{max, min},
	intrinsics::unlikely,
	mem::size_of,
};
use utils::{
	boxed::Box,
	collections::vec::Vec,
	errno,
	errno::{AllocResult, EResult},
	lock::Mutex,
	ptr::{arc::Arc, cow::Cow},
	TryClone,
};

// TODO count memory usage to enforce quota

/// The default maximum amount of memory the filesystem can use in bytes.
const DEFAULT_MAX_SIZE: usize = 512 * 1024 * 1024;
/// The maximum length of a name in the filesystem.
const MAX_NAME_LEN: usize = 255;

/// The content of a [`Node`].
#[derive(Debug)]
enum NodeContent {
	Regular(Vec<u8>),
	Directory(Vec<DirEntry<'static>>),
	Link(Vec<u8>),
	Fifo,
	Socket,
	BlockDevice { major: u32, minor: u32 },
	CharDevice { major: u32, minor: u32 },
}

#[derive(Debug)]
struct NodeInner {
	/// The file's permissions.
	mode: Mode,
	/// The number of links to the file.
	nlink: u16,
	/// The file owner's user ID.
	uid: Uid,
	/// The file owner's group ID.
	gid: Gid,
	/// Timestamp of the last modification of the metadata.
	ctime: Timestamp,
	/// Timestamp of the last modification of the file's content.
	mtime: Timestamp,
	/// Timestamp of the last access to the file.
	atime: Timestamp,
	/// The file's content.
	content: NodeContent,
}

impl NodeInner {
	/// Returns the [`Stat`] associated with the content.
	fn as_stat(&self) -> Stat {
		let (file_type, size, dev_major, dev_minor) = match &self.content {
			NodeContent::Regular(content) => (FileType::Regular, content.len() as _, 0, 0),
			NodeContent::Directory(_) => (FileType::Directory, 0, 0, 0),
			NodeContent::Link(target) => (FileType::Link, target.len() as _, 0, 0),
			NodeContent::Fifo => (FileType::Fifo, 0, 0, 0),
			NodeContent::Socket => (FileType::Socket, 0, 0, 0),
			NodeContent::BlockDevice {
				major,
				minor,
			} => (FileType::BlockDevice, 0, *major, *minor),
			NodeContent::CharDevice {
				major,
				minor,
			} => (FileType::CharDevice, 0, *major, *minor),
		};
		Stat {
			mode: file_type.to_mode() | self.mode,
			nlink: self.nlink,
			uid: self.uid,
			gid: self.gid,
			size,
			blocks: size / memory::PAGE_SIZE as u64,
			dev_major,
			dev_minor,
			ctime: self.ctime,
			mtime: self.mtime,
			atime: self.atime,
		}
	}
}

/// A tmpfs node.
#[derive(Clone, Debug)]
struct Node(Arc<Mutex<NodeInner>>);

impl Node {
	/// Creates a node from the given status.
	///
	/// Arguments:
	/// - `stat` is the status to initialize the node's with
	/// - `inode` is the inode of the node
	/// - `parent_inode` is the inode of the node's parent
	///
	/// Provided inodes are used only if the file is a directory, to create the `.` and `..`
	/// entries.
	pub fn new(
		stat: Stat,
		inode: Option<INode>,
		parent_inode: Option<INode>,
	) -> AllocResult<Self> {
		let file_type = stat.get_type().unwrap();
		let content = match file_type {
			FileType::Regular => NodeContent::Regular(Vec::new()),
			FileType::Directory => {
				let mut entries = Vec::new();
				if let Some(inode) = inode {
					entries.push(DirEntry {
						inode,
						entry_type: FileType::Directory,
						name: Cow::Borrowed(b"."),
					})?;
				}
				if let Some(parent_inode) = parent_inode {
					entries.push(DirEntry {
						inode: parent_inode,
						entry_type: FileType::Directory,
						name: Cow::Borrowed(b".."),
					})?;
				}
				NodeContent::Directory(entries)
			}
			FileType::Link => NodeContent::Link(Vec::new()),
			FileType::Fifo => NodeContent::Fifo,
			FileType::Socket => NodeContent::Socket,
			FileType::BlockDevice => NodeContent::BlockDevice {
				major: stat.dev_major,
				minor: stat.dev_minor,
			},
			FileType::CharDevice => NodeContent::CharDevice {
				major: stat.dev_major,
				minor: stat.dev_minor,
			},
		};
		let mut nlink = 1;
		if stat.get_type() == Some(FileType::Directory) {
			// Count the `.` entry
			nlink += 1;
		}
		Ok(Self(Arc::new(Mutex::new(NodeInner {
			mode: stat.mode,
			nlink,
			uid: stat.uid,
			gid: stat.gid,
			ctime: stat.ctime,
			mtime: stat.mtime,
			atime: stat.atime,
			content,
		}))?))
	}
}

impl NodeOps for Node {
	fn get_stat(&self, _loc: &FileLocation) -> EResult<Stat> {
		let inner = self.0.lock();
		Ok(inner.as_stat())
	}

	fn set_stat(&self, _loc: &FileLocation, set: StatSet) -> EResult<()> {
		let mut inner = self.0.lock();
		if let Some(mode) = set.mode {
			inner.mode = mode;
		}
		if let Some(nlink) = set.nlink {
			inner.nlink = nlink;
		}
		if let Some(uid) = set.uid {
			inner.uid = uid;
		}
		if let Some(gid) = set.gid {
			inner.gid = gid;
		}
		if let Some(ctime) = set.ctime {
			inner.ctime = ctime;
		}
		if let Some(mtime) = set.mtime {
			inner.mtime = mtime;
		}
		if let Some(atime) = set.atime {
			inner.atime = atime;
		}
		Ok(())
	}

	fn read_content(&self, _loc: &FileLocation, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let inner = self.0.lock();
		let content = match &inner.content {
			NodeContent::Regular(content) | NodeContent::Link(content) => content,
			NodeContent::Directory(_) => return Err(errno!(EISDIR)),
			_ => return Err(errno!(EINVAL)),
		};
		if off > content.len() as u64 {
			return Err(errno!(EINVAL));
		}
		let off = off as usize;
		let len = min(buf.len(), content.len() - off);
		buf[..len].copy_from_slice(&content[off..(off + len)]);
		Ok(len)
	}

	fn write_content(&self, _loc: &FileLocation, off: u64, buf: &[u8]) -> EResult<usize> {
		let mut inner = self.0.lock();
		match &mut inner.content {
			NodeContent::Regular(content) => {
				if off > content.len() as u64 {
					return Err(errno!(EINVAL));
				}
				let off = off as usize;
				let Some(end) = off.checked_add(buf.len()) else {
					return Err(errno!(EOVERFLOW));
				};
				let new_len = max(content.len(), end);
				content.resize(new_len, 0)?;
				content[off..].copy_from_slice(buf);
			}
			NodeContent::Link(content) => {
				content.resize(buf.len(), 0)?;
				content.copy_from_slice(buf);
			}
			NodeContent::Directory(_) => return Err(errno!(EISDIR)),
			_ => return Err(errno!(EINVAL)),
		}
		Ok(buf.len())
	}

	fn truncate_content(&self, _loc: &FileLocation, size: u64) -> EResult<()> {
		let mut inner = self.0.lock();
		let content = match &mut inner.content {
			NodeContent::Regular(content) => content,
			NodeContent::Directory(_) => return Err(errno!(EISDIR)),
			_ => return Err(errno!(EINVAL)),
		};
		content.truncate(size as _);
		Ok(())
	}

	fn entry_by_name<'n>(
		&self,
		loc: &FileLocation,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		let inner = self.0.lock();
		let NodeContent::Directory(entries) = &inner.content else {
			return Err(errno!(ENOTDIR));
		};
		let Some(off) = entries
			.binary_search_by(|ent| ent.name.as_ref().cmp(name))
			.ok()
		else {
			return Ok(None);
		};
		let ent = entries[off].try_clone()?;
		let fs = loc.get_filesystem().unwrap();
		let ops = fs.node_from_inode(ent.inode)?;
		Ok(Some((ent, ops)))
	}

	fn next_entry(
		&self,
		_loc: &FileLocation,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let inner = self.0.lock();
		let NodeContent::Directory(entries) = &inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Convert offset to `usize`
		let res = off
			.try_into()
			.ok()
			// Get entry
			.and_then(|off: usize| entries.get(off))
			.map(|ent| ent.try_clone())
			.transpose()
			// Add offset
			.map(|ent| ent.map(|entry| (entry, off + 1)));
		Ok(res?)
	}

	fn add_file(
		&self,
		parent: &FileLocation,
		name: &[u8],
		stat: Stat,
	) -> EResult<(INode, Box<dyn NodeOps>)> {
		let fs = parent.get_filesystem().unwrap();
		let fs = downcast_fs::<TmpFS>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let entry_type = stat.get_type().ok_or_else(|| errno!(EINVAL))?;
		let mut nodes = fs.nodes.lock();
		// Allocate a new slot. In case of later failure, this does not need rollback as the unused
		// slot is reused at the next call
		let (inode, slot) = nodes.get_free_slot()?;
		// Get parent entries
		let mut parent_inner = self.0.lock();
		let NodeContent::Directory(parent_entries) = &mut parent_inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Prepare node to be added
		let node = Node::new(stat, Some(inode), Some(parent.inode))?;
		// Add entry to parent
		let ent = DirEntry {
			inode,
			entry_type,
			name: Cow::Owned(name.try_into()?),
		};
		let res = parent_entries.binary_search_by(|ent| ent.name.as_ref().cmp(name));
		let Err(ent_index) = res else {
			return Err(errno!(EEXIST));
		};
		parent_entries.insert(ent_index, ent)?;
		// Insert node
		*slot = Some(node.clone());
		// Update links count
		if entry_type == FileType::Directory {
			parent_inner.nlink += 1;
		}
		Ok((inode, Box::new(node)?))
	}

	fn link(&self, parent: &FileLocation, name: &[u8], inode: INode) -> EResult<()> {
		let fs = parent.get_filesystem().unwrap();
		let fs = downcast_fs::<TmpFS>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		// Get node
		let node = fs.nodes.lock().get_node(inode)?.clone();
		let mut inner = node.0.lock();
		let entry_type = inner.as_stat().get_type().unwrap();
		let mut parent_inner = self.0.lock();
		// Get parent entries
		let NodeContent::Directory(parent_entries) = &mut parent_inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Insert the new entry
		let ent = DirEntry {
			inode,
			entry_type,
			name: Cow::Owned(name.try_into()?),
		};
		let res = parent_entries.binary_search_by(|ent| ent.name.as_ref().cmp(name));
		let Err(ent_index) = res else {
			return Err(errno!(EEXIST));
		};
		parent_entries.insert(ent_index, ent)?;
		// Update links count
		inner.nlink += 1;
		Ok(())
	}

	fn unlink(&self, parent: &FileLocation, name: &[u8]) -> EResult<()> {
		let fs = parent.get_filesystem().unwrap();
		let fs = downcast_fs::<TmpFS>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut parent_inner = self.0.lock();
		// Get parent entries
		let NodeContent::Directory(parent_entries) = &mut parent_inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Get entry to remove
		let ent_index = parent_entries
			.binary_search_by(|ent| ent.name.as_ref().cmp(name))
			.map_err(|_| errno!(ENOENT))?;
		let ent = &parent_entries[ent_index];
		// Get the entry's node
		let inode = ent.inode;
		let node = downcast_fs::<TmpFS>(fs)
			.nodes
			.lock()
			.get_node(inode)?
			.clone();
		let mut inner = node.0.lock();
		// If the node is a non-empty directory, error
		if !node.is_empty_directory(&FileLocation {
			mountpoint_id: parent.mountpoint_id,
			inode,
		})? {
			return Err(errno!(ENOTEMPTY));
		}
		// Remove entry
		parent_entries.remove(ent_index);
		// If the node is a directory, decrement the number of hard links to the parent
		// (because of the entry `..` in the removed node)
		if matches!(inner.content, NodeContent::Directory(_)) {
			parent_inner.nlink = parent_inner.nlink.saturating_sub(1);
		}
		inner.nlink = inner.nlink.saturating_sub(1);
		Ok(())
	}

	fn remove_file(&self, loc: &FileLocation) -> EResult<()> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<TmpFS>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut nodes = fs.nodes.lock();
		nodes.remove_node(loc.inode);
		Ok(())
	}
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
	nodes: Mutex<NodeStorage<Node>>,
}

impl TmpFS {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `max_size` is the maximum amount of memory the filesystem can use in bytes.
	/// - `readonly` tells whether the filesystem is readonly.
	pub fn new(max_size: usize, readonly: bool) -> EResult<Self> {
		let root = Node::new(
			Stat {
				mode: FileType::Directory.to_mode() | 0o1777,
				nlink: 0,
				uid: ROOT_UID,
				gid: ROOT_GID,
				size: 0,
				blocks: 0,
				dev_major: 0,
				dev_minor: 0,
				ctime: 0,
				mtime: 0,
				atime: 0,
			},
			Some(kernfs::ROOT_INODE),
			Some(kernfs::ROOT_INODE),
		)?;
		let fs = Self {
			max_size,
			// Size of the root node
			size: size_of::<Node>(),
			readonly,
			nodes: Mutex::new(NodeStorage::new(root)?),
		};
		Ok(fs)
	}
}

impl Filesystem for TmpFS {
	fn get_name(&self) -> &[u8] {
		b"tmpfs"
	}

	fn use_cache(&self) -> bool {
		false
	}

	fn get_root_inode(&self) -> INode {
		kernfs::ROOT_INODE
	}

	fn get_stat(&self) -> EResult<Statfs> {
		Ok(Statfs {
			f_type: 0,
			f_bsize: memory::PAGE_SIZE as _,
			f_blocks: 0,
			f_bfree: 0,
			f_bavail: 0,
			f_files: 0,
			f_ffree: 0,
			f_fsid: Default::default(),
			f_namelen: MAX_NAME_LEN as _,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn node_from_inode(&self, inode: INode) -> EResult<Box<dyn NodeOps>> {
		Ok(Box::new(self.nodes.lock().get_node(inode)?.clone())? as _)
	}
}

/// The tmpfs filesystem type.
pub struct TmpFsType;

impl FilesystemType for TmpFsType {
	fn get_name(&self) -> &'static [u8] {
		b"tmpfs"
	}

	fn detect(&self, _io: &dyn DeviceIO) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_io: Option<Arc<dyn DeviceIO>>,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<dyn Filesystem>> {
		Ok(Arc::new(TmpFS::new(DEFAULT_MAX_SIZE, readonly)?)?)
	}
}
