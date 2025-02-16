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
			downcast_fs, kernfs, kernfs::NodeStorage, FileOps, Filesystem, FilesystemOps,
			FilesystemType, NodeOps, StatSet, Statfs,
		},
		perm::{Gid, Uid, ROOT_GID, ROOT_UID},
		vfs,
		vfs::node::Node,
		DirContext, DirEntry, File, FileType, INode, Mode, Stat,
	},
	sync::mutex::Mutex,
	time::unit::Timestamp,
};
use core::{
	any::Any,
	cmp::{max, min},
	intrinsics::unlikely,
	mem::size_of,
};
use utils::{
	boxed::Box,
	collections::{path::PathBuf, vec::Vec},
	errno,
	errno::EResult,
	limits::PAGE_SIZE,
	ptr::{arc::Arc, cow::Cow},
};

// TODO count memory usage to enforce quota

/// The default maximum amount of memory the filesystem can use in bytes.
const DEFAULT_MAX_SIZE: usize = 512 * 1024 * 1024;
/// The maximum length of a name in the filesystem.
const MAX_NAME_LEN: usize = 255;

/// TmpFS directory entries.
#[derive(Debug)]
struct Dirent {
	/// The entry's inode
	inode: INode,
	/// Cached file type
	entry_type: FileType,
	/// The name of the entry
	name: Cow<'static, [u8]>,
}

/// The content of a [`TmpFSNode`].
#[derive(Debug)]
enum NodeContent {
	Regular(Vec<u8>),
	Directory(Vec<Dirent>),
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
	/// Returns the type of the file.
	fn get_type(&self) -> FileType {
		match &self.content {
			NodeContent::Regular(_) => FileType::Regular,
			NodeContent::Directory(_) => FileType::Directory,
			NodeContent::Link(_) => FileType::Link,
			NodeContent::Fifo => FileType::Fifo,
			NodeContent::Socket => FileType::Socket,
			NodeContent::BlockDevice {
				..
			} => FileType::BlockDevice,
			NodeContent::CharDevice {
				..
			} => FileType::CharDevice,
		}
	}

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
			blocks: size / PAGE_SIZE as u64,
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
struct TmpFSNode(Arc<Mutex<NodeInner>>);

impl TmpFSNode {
	/// Creates a node from the given status.
	///
	/// Arguments:
	/// - `stat` is the status to initialize the node's with
	/// - `inode` is the inode of the node
	/// - `parent_inode` is the inode of the node's parent
	///
	/// Provided inodes are used only if the file is a directory, to create the `.` and `..`
	/// entries.
	pub fn new(stat: &Stat, inode: Option<INode>, parent_inode: Option<INode>) -> EResult<Self> {
		let file_type = stat.get_type().ok_or_else(|| errno!(EINVAL))?;
		let content = match file_type {
			FileType::Regular => NodeContent::Regular(Vec::new()),
			FileType::Directory => {
				let mut entries = Vec::new();
				if let Some(inode) = inode {
					entries.push(Dirent {
						inode,
						entry_type: FileType::Directory,
						name: Cow::Borrowed(b"."),
					})?;
				}
				if let Some(parent_inode) = parent_inode {
					entries.push(Dirent {
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
		if file_type == FileType::Directory {
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

impl NodeOps for TmpFSNode {
	fn get_stat(&self, _node: &Node) -> EResult<Stat> {
		let inner = self.0.lock();
		Ok(inner.as_stat())
	}

	fn set_stat(&self, _node: &Node, set: StatSet) -> EResult<()> {
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

	fn lookup_entry(&self, dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*dir.fs.ops);
		let inner = self.0.lock();
		let NodeContent::Directory(entries) = &inner.content else {
			return Err(errno!(ENOTDIR));
		};
		ent.node = entries
			.binary_search_by(|ent| ent.name.as_ref().cmp(&ent.name))
			.ok()
			.map(|inode| -> EResult<_> {
				let ent = &entries[inode];
				let node = fs.nodes.lock().get_node(ent.inode)?.clone();
				let node = Arc::new(Node {
					inode: ent.inode as _,
					fs: dir.fs.clone(),
					node_ops: Box::new(node)?,
					file_ops: Box::new(TmpFSFile)?,
					pages: Default::default(),
				})?;
				Ok(node)
			})
			.transpose()?;
		Ok(())
	}

	fn iter_entries(&self, _dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let inner = self.0.lock();
		let NodeContent::Directory(entries) = &inner.content else {
			return Err(errno!(ENOTDIR));
		};
		let off: usize = ctx.off.try_into().map_err(|_| errno!(EOVERFLOW))?;
		let iter = entries.iter().skip(off);
		for e in iter {
			ctx.off += 1;
			let ent = DirEntry {
				inode: e.inode,
				entry_type: e.entry_type,
				name: &e.name,
			};
			if !(*ctx.write)(&ent)? {
				break;
			}
		}
		Ok(())
	}

	fn link(&self, parent: &Node, name: &[u8], inode: INode) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		// Get node
		let node = fs.nodes.lock().get_node(inode)?.clone();
		let mut inner = node.0.lock();
		let mut parent_inner = self.0.lock();
		// Get parent entries
		let NodeContent::Directory(parent_entries) = &mut parent_inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Insert the new entry
		let ent = Dirent {
			inode,
			entry_type: inner.get_type(),
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

	fn unlink(&self, parent: &Node, name: &[u8]) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
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
		let node = downcast_fs::<TmpFS>(fs)
			.nodes
			.lock()
			.get_node(ent.inode)?
			.clone();
		// If the node is a non-empty directory, error
		if matches!(&node.0.lock().content, NodeContent::Directory(ents) if !ents.is_empty()) {
			return Err(errno!(ENOTEMPTY));
		}
		// Remove entry
		parent_entries.remove(ent_index);
		// If the node is a directory, decrement the number of hard links to the parent
		// (because of the entry `..` in the removed node)
		let mut inner = node.0.lock();
		if matches!(inner.content, NodeContent::Directory(_)) {
			parent_inner.nlink = parent_inner.nlink.saturating_sub(1);
		}
		inner.nlink = inner.nlink.saturating_sub(1);
		Ok(())
	}
}

/// Get [`NodeInner`] from [`File`].
fn file_to_node(file: &File) -> &Mutex<NodeInner> {
	let node_ops = &*file.node().unwrap().node_ops;
	let node: &TmpFSNode = (node_ops as &dyn Any).downcast_ref().unwrap();
	&node.0
}

/// Open file operations.
#[derive(Debug)]
pub struct TmpFSFile;

impl FileOps for TmpFSFile {
	fn read(&self, file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let inner = file_to_node(file).lock();
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

	fn write(&self, file: &File, off: u64, buf: &[u8]) -> EResult<usize> {
		let mut inner = file_to_node(file).lock();
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
				content[off..end].copy_from_slice(buf);
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

	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let mut inner = file_to_node(file).lock();
		let content = match &mut inner.content {
			NodeContent::Regular(content) => content,
			NodeContent::Directory(_) => return Err(errno!(EISDIR)),
			_ => return Err(errno!(EINVAL)),
		};
		content.truncate(size as _);
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
	nodes: Mutex<NodeStorage<TmpFSNode>>,
}

impl TmpFS {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `max_size` is the maximum amount of memory the filesystem can use in bytes.
	/// - `readonly` tells whether the filesystem is readonly.
	pub fn new(max_size: usize, readonly: bool) -> EResult<Self> {
		let root = TmpFSNode::new(
			&Stat {
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
			size: size_of::<TmpFSNode>(),
			readonly,
			nodes: Mutex::new(NodeStorage::new(root)?),
		};
		Ok(fs)
	}
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
			f_namelen: MAX_NAME_LEN as _,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn root(&self, fs: Arc<Filesystem>) -> EResult<Arc<Node>> {
		let node = self.nodes.lock().get_node(kernfs::ROOT_INODE)?.clone();
		Ok(Arc::new(Node {
			inode: 0,
			fs,
			node_ops: Box::new(node)?,
			file_ops: Box::new(TmpFSFile)?,
			pages: Default::default(),
		})?)
	}

	fn create_node(&self, fs: Arc<Filesystem>, stat: &Stat) -> EResult<Arc<Node>> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		let mut nodes = self.nodes.lock();
		let (inode, slot) = nodes.get_free_slot()?;
		let node = TmpFSNode::new(stat, Some(inode), None)?;
		*slot = Some(node.clone());
		let node = Arc::new(Node {
			inode,
			fs,
			node_ops: Box::new(node)?,
			file_ops: Box::new(TmpFSFile)?,
			pages: Default::default(),
		})?;
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

	fn detect(&self, _io: &dyn DeviceIO) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_io: Option<Arc<dyn DeviceIO>>,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Box<dyn FilesystemOps>> {
		Ok(Box::new(TmpFS::new(DEFAULT_MAX_SIZE, readonly)?)?)
	}
}
