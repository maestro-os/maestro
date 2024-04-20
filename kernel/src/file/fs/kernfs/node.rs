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

//! KernFS node utilities.

use crate::{
	file::{
		fs::{kernfs::KernFS, Filesystem, NodeOps, StatSet},
		perm::{Gid, Uid},
		DirEntry, FileType, INode, Mode, Stat,
	},
	memory,
	time::unit::Timestamp,
};
use core::{
	any::Any,
	cmp::{max, min},
	fmt,
	fmt::{Debug, Write},
};
use utils::{
	boxed::Box,
	collections::vec::Vec,
	errno,
	errno::{AllocResult, EResult},
	lock::Mutex,
	ptr::cow::Cow,
	DisplayableStr, TryClone,
};

/// Downcasts the given `fs` into [`KernFS`].
fn downcast_fs(fs: &dyn Filesystem) -> &KernFS {
	(fs as &dyn Any).downcast_ref().unwrap()
}

/// A node that is owned by the kernfs, or another entity (such as another node).
///
/// An `OwnedNode` *may* or *may not* have an associated inode.
pub trait OwnedNode: NodeOps {
	/// Returns the operations handle for the node to be used outside the kernfs.
	///
	/// This handle can carry additional information, but it is recommended to use a ZST if
	/// possible so that no memory allocation has to be made.
	fn detached(&self) -> AllocResult<Box<dyn NodeOps>>;
}

/// The content of a [`DefaultNode`].
#[derive(Debug)]
enum DefaultNodeContent {
	Regular(Vec<u8>),
	Directory(Vec<DirEntry<'static>>),
	Link(Vec<u8>),
	Fifo,
	Socket,
	BlockDevice { major: u32, minor: u32 },
	CharDevice { major: u32, minor: u32 },
}

#[derive(Debug)]
struct DefaultNodeInner {
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
	content: DefaultNodeContent,
}

impl DefaultNodeInner {
	/// Returns the [`Stat`] associated with the content.
	fn as_stat(&self) -> Stat {
		let (file_type, size, dev_major, dev_minor) = match &self.content {
			DefaultNodeContent::Regular(content) => (FileType::Regular, content.len() as _, 0, 0),
			DefaultNodeContent::Directory(_) => (FileType::Directory, 0, 0, 0),
			DefaultNodeContent::Link(target) => (FileType::Link, target.len() as _, 0, 0),
			DefaultNodeContent::Fifo => (FileType::Fifo, 0, 0, 0),
			DefaultNodeContent::Socket => (FileType::Socket, 0, 0, 0),
			DefaultNodeContent::BlockDevice {
				major,
				minor,
			} => (FileType::BlockDevice, 0, *major, *minor),
			DefaultNodeContent::CharDevice {
				major,
				minor,
			} => (FileType::CharDevice, 0, *major, *minor),
		};
		Stat {
			file_type,
			mode: self.mode,
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

/// A kernfs node with the default behaviour for each file type.
#[derive(Debug)]
pub struct DefaultNode(Mutex<DefaultNodeInner>);

impl DefaultNode {
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
		let content = match stat.file_type {
			FileType::Regular => DefaultNodeContent::Regular(Vec::new()),
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
				DefaultNodeContent::Directory(entries)
			}
			FileType::Link => DefaultNodeContent::Link(Vec::new()),
			FileType::Fifo => DefaultNodeContent::Fifo,
			FileType::Socket => DefaultNodeContent::Socket,
			FileType::BlockDevice => DefaultNodeContent::BlockDevice {
				major: stat.dev_major,
				minor: stat.dev_minor,
			},
			FileType::CharDevice => DefaultNodeContent::CharDevice {
				major: stat.dev_major,
				minor: stat.dev_minor,
			},
		};
		let mut nlink = 1;
		if stat.file_type == FileType::Directory {
			// Count the `.` entry
			nlink += 2;
		}
		Ok(Self(Mutex::new(DefaultNodeInner {
			mode: stat.mode,
			nlink,
			uid: stat.uid,
			gid: stat.gid,
			ctime: stat.ctime,
			mtime: stat.mtime,
			atime: stat.atime,
			content,
		})))
	}
}

impl OwnedNode for DefaultNode {
	fn detached(&self) -> AllocResult<Box<dyn NodeOps>> {
		Ok(Box::new(DefaultNodeOps)? as _)
	}
}

impl NodeOps for DefaultNode {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		let inner = self.0.lock();
		Ok(inner.as_stat())
	}

	fn set_stat(&self, _inode: INode, _fs: &dyn Filesystem, set: StatSet) -> EResult<()> {
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

	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<(u64, bool)> {
		let inner = self.0.lock();
		// Get content
		let content = match &inner.content {
			DefaultNodeContent::Regular(content) => content,
			DefaultNodeContent::Directory(_) => return Err(errno!(EISDIR)),
			_ => return Err(errno!(EINVAL)),
		};
		// Validation
		if off > content.len() as u64 {
			return Err(errno!(EINVAL));
		}
		// Copy
		let off = off as usize;
		let len = min(buf.len(), content.len() - off);
		buf[..len].copy_from_slice(&content[off..(off + len)]);
		let eof = (off + len) >= content.len();
		Ok((len as _, eof))
	}

	fn write_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &[u8],
	) -> EResult<u64> {
		let mut inner = self.0.lock();
		// Get content
		let content = match &mut inner.content {
			DefaultNodeContent::Regular(content) => content,
			DefaultNodeContent::Directory(_) => return Err(errno!(EISDIR)),
			_ => return Err(errno!(EINVAL)),
		};
		// Validation
		if off > content.len() as u64 {
			return Err(errno!(EINVAL));
		}
		let off = off as usize;
		// Allocation
		let new_len = max(content.len(), off + buf.len());
		content.resize(new_len, 0)?;
		// Copy
		content[off..(off + buf.len())].copy_from_slice(buf);
		Ok(buf.len() as _)
	}

	fn truncate_content(&self, _inode: INode, _fs: &dyn Filesystem, size: u64) -> EResult<()> {
		let mut inner = self.0.lock();
		let content = match &mut inner.content {
			DefaultNodeContent::Regular(content) => content,
			DefaultNodeContent::Directory(_) => return Err(errno!(EISDIR)),
			_ => return Err(errno!(EINVAL)),
		};
		content.truncate(size as _);
		Ok(())
	}

	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		let inner = self.0.lock();
		let DefaultNodeContent::Directory(entries) = &inner.content else {
			return Err(errno!(ENOTDIR));
		};
		let Some(off) = entries
			.binary_search_by(|ent| ent.name.as_ref().cmp(name))
			.ok()
		else {
			return Ok(None);
		};
		let ent = entries[off].try_clone()?;
		let ops = fs.node_from_inode(ent.inode)?;
		Ok(Some((ent, ops)))
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let inner = self.0.lock();
		let DefaultNodeContent::Directory(entries) = &inner.content else {
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
		parent_inode: INode,
		fs: &dyn Filesystem,
		name: &[u8],
		stat: Stat,
	) -> EResult<(INode, Box<dyn NodeOps>)> {
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}
		let fs = downcast_fs(fs);
		let mut nodes = fs.nodes.lock();
		// Allocate a new slot. In case of later failure, this does not need rollback as the unused
		// slot is reused at the next call
		let (inode, slot) = nodes.get_free_slot()?;
		// Get parent entries
		let mut parent_inner = self.0.lock();
		let DefaultNodeContent::Directory(parent_entries) = &mut parent_inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Prepare node to be added
		let node = Box::new(DefaultNode::new(stat, Some(inode), Some(parent_inode))?)?;
		let file_type = node.0.lock().as_stat().file_type;
		let ops = node.detached()?;
		// Add entry to parent
		let ent = DirEntry {
			inode,
			entry_type: file_type,
			name: Cow::Owned(name.try_into()?),
		};
		let res = parent_entries.binary_search_by(|ent| ent.name.as_ref().cmp(name));
		let Err(ent_index) = res else {
			return Err(errno!(EEXIST));
		};
		parent_entries.insert(ent_index, ent)?;
		// Insert node
		*slot = Some(node);
		// Update links count
		if file_type == FileType::Directory {
			parent_inner.nlink += 1;
		}
		Ok((inode, ops))
	}

	fn add_link(
		&self,
		_parent_inode: INode,
		fs: &dyn Filesystem,
		name: &[u8],
		inode: INode,
	) -> EResult<()> {
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}
		// Get node
		let node = {
			// Get a detached version to make sure `get_stat` does not cause a deadlock
			let fs = downcast_fs(fs);
			let nodes = fs.nodes.lock();
			nodes.get_node(inode)?.detached()?
		};
		let stat = node.get_stat(inode, fs)?;
		let mut parent_inner = self.0.lock();
		// Get parent entries
		let DefaultNodeContent::Directory(parent_entries) = &mut parent_inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Insert the new entry
		let ent = DirEntry {
			inode,
			entry_type: stat.file_type,
			name: Cow::Owned(name.try_into()?),
		};
		let res = parent_entries.binary_search_by(|ent| ent.name.as_ref().cmp(name));
		let Err(ent_index) = res else {
			return Err(errno!(EEXIST));
		};
		parent_entries.insert(ent_index, ent)?;
		// Update links count
		node.set_stat(
			inode,
			fs,
			StatSet {
				nlink: Some(stat.nlink + 1),
				..Default::default()
			},
		)?;
		Ok(())
	}

	fn remove_file(
		&self,
		_parent_inode: INode,
		fs: &dyn Filesystem,
		name: &[u8],
	) -> EResult<(u16, INode)> {
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}
		let fs = downcast_fs(fs);
		let mut parent_inner = self.0.lock();
		// Get parent entries
		let DefaultNodeContent::Directory(parent_entries) = &mut parent_inner.content else {
			return Err(errno!(ENOTDIR));
		};
		// Get entry to remove
		let ent_index = parent_entries
			.binary_search_by(|ent| ent.name.as_ref().cmp(name))
			.map_err(|_| errno!(ENOENT))?;
		let ent = &parent_entries[ent_index];
		let inode = ent.inode;
		// Get the entry's node
		let node = {
			// Get a detached version to make sure `get_stat` does not cause a deadlock
			let nodes = fs.nodes.lock();
			nodes.get_node(inode)?.detached()?
		};
		let stat = node.get_stat(inode, fs)?;
		// If the node is a non-empty directory, error
		if !node.is_empty_directory(inode, fs)? {
			return Err(errno!(ENOTEMPTY));
		}
		// Remove entry
		parent_entries.remove(ent_index);
		// If the node is a directory, decrement the number of hard links to the parent
		// (because of the entry `..` in the removed node)
		if stat.file_type == FileType::Directory {
			parent_inner.nlink = parent_inner.nlink.saturating_sub(1);
		}
		let links = stat.nlink.saturating_sub(1);
		node.set_stat(
			inode,
			fs,
			StatSet {
				nlink: Some(links),
				..Default::default()
			},
		)?;
		// If no link is left, remove the node
		if links == 0 {
			let mut nodes = fs.nodes.lock();
			nodes.remove_node(inode);
		}
		Ok((links, inode))
	}
}

/// Operations for [`DefaultNode`].
#[derive(Debug)]
pub struct DefaultNodeOps;

// This implementation only forwards to the actual node.
impl NodeOps for DefaultNodeOps {
	fn get_stat(&self, inode: INode, fs: &dyn Filesystem) -> EResult<Stat> {
		let fs = downcast_fs(fs);
		let nodes = fs.nodes.lock();
		let node = nodes.get_node(inode)?;
		node.get_stat(inode, fs)
	}

	fn set_stat(&self, inode: INode, fs: &dyn Filesystem, set: StatSet) -> EResult<()> {
		let fs = downcast_fs(fs);
		let nodes = fs.nodes.lock();
		let node = nodes.get_node(inode)?;
		node.set_stat(inode, fs, set)
	}

	fn read_content(
		&self,
		inode: INode,
		fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<(u64, bool)> {
		let fs = downcast_fs(fs);
		let nodes = fs.nodes.lock();
		let node = nodes.get_node(inode)?;
		node.read_content(inode, fs, off, buf)
	}

	fn write_content(
		&self,
		inode: INode,
		fs: &dyn Filesystem,
		off: u64,
		buf: &[u8],
	) -> EResult<u64> {
		let fs = downcast_fs(fs);
		let nodes = fs.nodes.lock();
		let node = nodes.get_node(inode)?;
		node.write_content(inode, fs, off, buf)
	}

	fn truncate_content(&self, inode: INode, fs: &dyn Filesystem, size: u64) -> EResult<()> {
		let fs = downcast_fs(fs);
		let nodes = fs.nodes.lock();
		let node = nodes.get_node(inode)?;
		node.truncate_content(inode, fs, size)
	}

	fn entry_by_name<'n>(
		&self,
		inode: INode,
		fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		let fs = downcast_fs(fs);
		let nodes = fs.nodes.lock();
		let node = nodes.get_node(inode)?;
		node.entry_by_name(inode, fs, name)
	}

	fn next_entry(
		&self,
		inode: INode,
		fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let fs = downcast_fs(fs);
		let nodes = fs.nodes.lock();
		let node = nodes.get_node(inode)?;
		node.next_entry(inode, fs, off)
	}
}

/// Writer for [`format_content_args`].
#[derive(Debug)]
struct FormatContentWriter<'b> {
	src_cursor: u64,
	dst: &'b mut [u8],
	dst_cursor: usize,
	eof: bool,
}

impl<'b> Write for FormatContentWriter<'b> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		if s.is_empty() {
			return Ok(());
		}
		let chunk = s.as_bytes();
		// If at least part of the chunk is inside the range to read, copy
		if (chunk.len() as u64) > self.src_cursor {
			self.eof = false;
			// If the end of the output buffer is reached, stop
			if self.dst_cursor >= self.dst.len() {
				return Err(fmt::Error);
			}
			// Offset and size of the range in the chunk to copy
			let src_cursor = self.src_cursor as usize;
			let size = min(
				self.dst.len().saturating_sub(self.dst_cursor),
				chunk.len().saturating_sub(src_cursor),
			);
			self.dst[self.dst_cursor..(self.dst_cursor + size)]
				.copy_from_slice(&chunk[src_cursor..(src_cursor + size)]);
			self.dst_cursor += size;
			// If the end of the chunk is reached, set `eof` if necessary
			if src_cursor + size >= chunk.len() {
				// If other non-empty chunks remain, the next iteration will cancel this
				self.eof = true;
			}
		}
		// Update cursor
		self.src_cursor = self.src_cursor.saturating_sub(chunk.len() as _);
		Ok(())
	}
}

/// Implementation of [`crate::format_content`].
pub fn format_content_args(
	off: u64,
	buf: &mut [u8],
	args: fmt::Arguments<'_>,
) -> EResult<(u64, bool)> {
	let mut writer = FormatContentWriter {
		src_cursor: off,
		dst: buf,
		dst_cursor: 0,
		eof: true,
	};
	let res = fmt::write(&mut writer, args);
	if res.is_err() && (writer.dst_cursor < writer.dst.len()) {
		panic!("a formatting trait implementation returned an error");
	}
	Ok((writer.dst_cursor as _, writer.eof))
}

/// Formats the content of a kernfs node and write it on a buffer.
///
/// This is meant to be used in [`NodeOps::read_content`].
///
/// `off` and `buf` are the corresponding arguments from [`NodeOps::read_content`].
#[macro_export]
macro_rules! format_content {
    ($off:expr, $buf:expr, $($arg:tt)*) => {{
		$crate::file::fs::kernfs::node::format_content_args($off, $buf, format_args!($($arg)*))
	}};
}

/// A static symbolic link pointing to a constant target.
#[derive(Debug, Default)]
pub struct StaticLink<const TARGET: &'static [u8]>;

impl<const TARGET: &'static [u8]> NodeOps for StaticLink<TARGET> {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		Ok(Stat {
			file_type: FileType::Link,
			mode: 0o777,
			..Default::default()
		})
	}

	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<(u64, bool)> {
		format_content!(off, buf, "{}", DisplayableStr(TARGET))
	}
}

/// A builder for an entry of a [`StaticDir`].
///
/// `T` is the type of the parameter passed to `init`.
#[derive(Debug)]
pub struct StaticEntryBuilder<T = ()> {
	/// The name of the entry.
	pub name: &'static [u8],
	/// The type of the entry.
	pub entry_type: FileType,
	/// A builder which returns a handle to perform operations on the node.
	pub init: fn(T) -> AllocResult<Box<dyn NodeOps>>,
}

/// Helper to initialize an entry handle, using [`Default`].
pub fn entry_init_default<'e, E: 'e + NodeOps + Default>(
	_: (),
) -> AllocResult<Box<dyn 'e + NodeOps>> {
	box_wrap(E::default())
}

/// Helper to initialize an entry handle, using [`From`].
pub fn entry_init_from<'e, E: 'e + NodeOps + From<T>, T>(
	val: T,
) -> AllocResult<Box<dyn 'e + NodeOps>> {
	box_wrap(E::from(val))
}

/// Helper to wrap a [`NodeOps`] into a [`Box`].
pub fn box_wrap<'n, N: 'n + NodeOps>(ops: N) -> AllocResult<Box<dyn 'n + NodeOps>> {
	Ok(Box::new(ops)? as _)
}

// TODO: the day Rust supports `dyn` in const generics (if it ever does), replace the entries array
// by a const generic
/// A read-only virtual directory used to point to other nodes.
#[derive(Debug)]
pub struct StaticDir<T: 'static + Clone + Debug = ()> {
	/// The directory's entries, sorted alphabeticaly by name.
	///
	/// **Warning**: If this array is not sorted correctly, the behaviour of
	/// [`NodeOps::entry_by_name`] is undefined.
	pub entries: &'static [StaticEntryBuilder<T>],
	/// Data used to initialize sub-nodes.
	pub data: T,
}

impl<T: 'static + Clone + Debug> StaticDir<T> {
	/// Inner implementation of [`Self::entry_by_name`].
	pub fn entry_by_name_inner<'n>(
		&self,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		let Ok(index) = self.entries.binary_search_by(|e| e.name.cmp(name)) else {
			return Ok(None);
		};
		let e = &self.entries[index];
		let ops = (e.init)(self.data.clone())?;
		Ok(Some((
			DirEntry {
				inode: 0,
				entry_type: e.entry_type,
				name: Cow::Borrowed(name),
			},
			ops,
		)))
	}

	/// Inner implementation of [`Self::next_entry`].
	pub fn next_entry_inner(&self, off: u64) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let Some(e) = self.entries.get(off) else {
			return Ok(None);
		};
		Ok(Some((
			DirEntry {
				inode: 0,
				entry_type: e.entry_type,
				name: Cow::Borrowed(e.name),
			},
			(off + 1) as _,
		)))
	}
}

impl<T: 'static + Clone + Debug> NodeOps for StaticDir<T> {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		Ok(Stat {
			file_type: FileType::Directory,
			mode: 0o555,
			..Default::default()
		})
	}

	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		self.entry_by_name_inner(name)
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		self.next_entry_inner(off)
	}
}

#[cfg(test)]
mod test {
	#[test_case]
	fn content_chunks() {
		let val = 123;
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(0, &mut out, "{} {} {}", val, "def", "ghi").unwrap();
		assert_eq!(out.as_slice(), b"123 def g");
		assert_eq!(len, 9);
		assert!(!eof);
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(9, &mut out, "{} {} {}", "abc", "def", val).unwrap();
		assert_eq!(out.as_slice(), b"23\0\0\0\0\0\0\0");
		assert_eq!(len, 2);
		assert!(eof);
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(3, &mut out, "{} {} {}", "abc", val, "ghi").unwrap();
		assert_eq!(out.as_slice(), b" 123 ghi\0");
		assert_eq!(len, 8);
		assert!(eof);
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(4, &mut out, "{} {} {}", val, "def", "ghi").unwrap();
		assert_eq!(out.as_slice(), b"def ghi\0\0");
		assert_eq!(len, 7);
		assert!(eof);
		let mut out = [0u8; 5];
		let (len, eof) = format_content!(0, &mut out, "{} {} {}", "abc", val, "ghi").unwrap();
		assert_eq!(out.as_slice(), b"abc 1");
		assert_eq!(len, 5);
		assert!(!eof);
	}
}
