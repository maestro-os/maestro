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

//! A filesystem is the representation of the file hierarchy on a storage
//! device.

pub mod ext2;
pub mod initramfs;
pub mod kernfs;
pub mod proc;
pub mod tmp;

use super::{
	perm::{Gid, Uid},
	vfs, DirContext, File, INode, Mode, Stat,
};
use crate::{
	device::BlkDev,
	file::vfs::node::Node,
	memory::{cache::RcFrame, user::UserSlice},
	sync::mutex::Mutex,
	syscall::ioctl,
	time::unit::Timestamp,
};
use core::{
	any::Any,
	borrow::Borrow,
	cmp::min,
	ffi::{c_int, c_void},
	fmt,
	fmt::{Debug, Formatter},
	hash::{Hash, Hasher},
	intrinsics::unlikely,
};
use utils::{
	boxed::Box,
	collections::{hashmap::HashMap, hashset::HashSet, path::PathBuf, string::String},
	errno,
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
};

/// Used in the f_fsid field of [`Statfs`].
///
/// It is currently unused.
#[repr(C)]
#[derive(Debug, Default)]
struct Fsid {
	/// Unused.
	_val: [c_int; 2],
}

/// Statistics about a filesystem.
#[repr(C)]
#[derive(Debug)]
pub struct Statfs {
	/// Type of filesystem.
	f_type: u32,
	/// Optimal transfer block size.
	f_bsize: u32,
	/// Total data blocks in filesystem.
	f_blocks: i64,
	/// Free blocks in filesystem.
	f_bfree: i64,
	/// Free blocks available to unprivileged user.
	f_bavail: i64,
	/// Total inodes in filesystem.
	f_files: i64,
	/// Free inodes in filesystem.
	f_ffree: i64,
	/// Filesystem ID.
	f_fsid: Fsid,
	/// Maximum length of filenames.
	f_namelen: u32,
	/// Fragment size.
	f_frsize: u32,
	/// Mount flags of filesystem.
	f_flags: u32,
}

/// A set of attributes to modify on a file's status.
#[derive(Default)]
pub struct StatSet {
	/// Set the mode of the file.
	pub mode: Option<Mode>,
	/// Set the owner's user ID.
	pub uid: Option<Uid>,
	/// Set the owner's group ID.
	pub gid: Option<Gid>,
	/// Set the timestamp of the last modification of the metadata.
	pub ctime: Option<Timestamp>,
	/// Set the timestamp of the last modification of the file's content.
	pub mtime: Option<Timestamp>,
	/// Set the timestamp of the last access to the file.
	pub atime: Option<Timestamp>,
}

/// Filesystem node operations.
pub trait NodeOps: Any + Debug {
	/// Looks for an entry in `dir` with the name in `ent`. If found, the function sets the
	/// corresponding [`Node`] in `ent`.
	///
	/// If the entry does not exist, the function set the node to `None`.
	///
	/// If the node is not a directory, the function returns [`errno::ENOTDIR`].
	///
	/// The default implementation of this function returns an error.
	fn lookup_entry(&self, dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let _ = (dir, ent);
		Err(errno!(ENOTDIR))
	}

	/// Iterates on the entries of the directory `dir`.
	///
	/// If the node is not a directory, the function returns [`errno::ENOTDIR`].
	///
	/// The default implementation of this function returns an error.
	fn iter_entries(&self, dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let _ = (dir, ctx);
		Err(errno!(ENOTDIR))
	}

	/// Adds a hard link into the directory.
	///
	/// Arguments:
	/// - `parent` is the location of the parent directory
	/// - `ent` is the entry to add
	///
	/// If this feature is not supported by the filesystem, the function returns
	/// an error.
	///
	/// The default implementation of this function returns an error.
	fn link(&self, parent: Arc<Node>, ent: &vfs::Entry) -> EResult<()> {
		let _ = (parent, ent);
		Err(errno!(ENOTDIR))
	}

	/// Removes a hard link from the directory.
	///
	/// Arguments:
	/// - `parent` is the parent directory
	/// - `ent` is the hard link to remove
	///
	/// On success, the function returns the number of links to the target node left, along with
	/// the target inode.
	///
	/// If the file to be removed is a non-empty directory, the function returns
	/// [`errno::ENOTEMPTY`].
	///
	/// If this feature is not supported by the filesystem, the function returns
	/// an error.
	///
	/// The default implementation of this function returns an error.
	fn unlink(&self, parent: &Node, ent: &vfs::Entry) -> EResult<()> {
		let _ = (parent, ent);
		Err(errno!(ENOTDIR))
	}

	/// Reads the path the symbolic link points to and writes it into `buf`.
	/// If the actual path is larger than the provided buffer, it is truncated.
	///
	/// On success, the function returns the number of bytes read.
	///
	/// If the node is not a symbolic link, the function returns [`errno::EINVAL`].
	///
	/// If this feature is not supported by the filesystem, the function returns
	/// an error.
	///
	/// The default implementation of this function returns an error.
	fn readlink(&self, node: &Node, buf: UserSlice<u8>) -> EResult<usize> {
		let _ = (node, buf);
		Err(errno!(EINVAL))
	}

	/// Writes the path the symbolic link points to and writes it into `buf`.
	///
	/// If the node is not a symbolic link, the function returns [`errno::EINVAL`].
	///
	/// **Note**: this function must be called **only** for the creation of the symbolic link.
	/// After being created, the content is immutable.
	///
	/// If this feature is not supported by the filesystem, the function returns
	/// an error.
	///
	/// The default implementation of this function returns an error.
	fn writelink(&self, node: &Node, buf: &[u8]) -> EResult<()> {
		let _ = (node, buf);
		Err(errno!(EINVAL))
	}

	/// Renames or moves a file on the filesystem.
	///
	/// If this feature is not supported by the filesystem, the function returns
	/// an error.
	///
	/// The default implementation of this function returns an error.
	fn rename(
		&self,
		old_entry: &vfs::Entry,
		new_parent: &vfs::Entry,
		new_name: &[u8],
	) -> EResult<()> {
		let _ = (old_entry, new_parent, new_name);
		Err(errno!(EINVAL))
	}

	/// Reads a page at offset `off` in pages, from `node`.
	///
	/// First, the function attempts to read the page from the node's page cache. If not present,
	/// then it is read from disk.
	///
	/// The default implementation of this function returns an error.
	fn read_page(&self, node: &Arc<Node>, off: u64) -> EResult<RcFrame> {
		let _ = (node, off);
		Err(errno!(EINVAL))
	}

	/// Writes the frame `frame` back to storage.
	///
	/// The default implementation of this function returns an error.
	fn write_frame(&self, node: &Node, frame: &RcFrame) -> EResult<()> {
		let _ = (node, frame);
		Err(errno!(EINVAL))
	}

	/// Updates the node's status back to disk.
	///
	/// The default implementation of this function does nothing.
	fn sync_stat(&self, node: &Node) -> EResult<()> {
		let _ = node;
		Ok(())
	}
}

/// Open file operations.
///
/// This trait is separated so that files with a special behavior can be handled. As an example,
/// *device files*, *pipes* or *sockets* have a behavior that is independent of the underlying
/// filesystem.
pub trait FileOps: Any + Debug {
	/// Returns the file's status.
	///
	/// This function **MUST** be overridden when there is no [`Node`] associated with `file`.
	fn get_stat(&self, file: &File) -> EResult<Stat> {
		let node = file.vfs_entry.as_ref().unwrap().node();
		let stat = node.stat.lock().clone();
		Ok(stat)
	}

	/// Increments the reference counter.
	fn acquire(&self, file: &File) {
		let _ = file;
	}

	/// Decrements the reference counter.
	fn release(&self, file: &File) {
		let _ = file;
	}

	/// Wait for events on the file.
	///
	/// Arguments:
	/// - `file` is the file to perform the operation onto
	/// - `mask` is the mask of events to wait for
	///
	/// On success, the function returns the mask events that occurred.
	fn poll(&self, file: &File, mask: u32) -> EResult<u32> {
		let _ = (file, mask);
		Err(errno!(EINVAL))
	}

	/// Performs an ioctl operation on the device file.
	///
	/// Arguments:
	/// - `file` is the file to perform the operation onto
	/// - `request` is the ID of the request to perform
	/// - `argp` is a pointer to the argument
	fn ioctl(&self, file: &File, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		let _ = (file, request, argp);
		Err(errno!(EINVAL))
	}

	/// Reads from the content of `file` into the buffer `buf`.
	///
	/// Arguments:
	/// - `file` is the location of the file
	/// - `off` is the offset from which the data will be read from the node's data
	/// - `buf` is the buffer in which the data is to be written
	///
	/// On success, the function returns the number of bytes read.
	///
	/// The default implementation of this function returns an error.
	fn read(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let _ = (file, off, buf);
		Err(errno!(EINVAL))
	}

	/// Writes to the content of `file` from the buffer `buf`.
	///
	/// Arguments:
	/// - `file` is the file
	/// - `off` is the offset at which the data will be written in the node's data
	/// - `buf` is the buffer in which the data is to be read from
	///
	/// On success, the function returns the number of bytes written.
	///
	/// The default implementation of this function returns an error.
	fn write(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let _ = (file, off, buf);
		Err(errno!(EINVAL))
	}

	/// Changes the size of the file, truncating its content if necessary.
	///
	/// If `size` is greater than or equals to the current size of the file, the function does
	/// nothing.
	///
	/// The default implementation of this function returns an error.
	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let _ = (file, size);
		Err(errno!(EINVAL))
	}
}

/// Generic implementation for [`FileOps::read`] on regular files.
///
/// **Note**: `file` **must** have an associated [`Node`], otherwise the function panics.
pub fn generic_file_read(file: &File, mut off: u64, buf: UserSlice<u8>) -> EResult<usize> {
	let node = file.node().unwrap();
	let size = file.stat()?.size;
	if unlikely(off > size) {
		return Err(errno!(EINVAL));
	}
	let buf_len = min(buf.len() as u64, size - off);
	let start = off / PAGE_SIZE as u64;
	let end = off.saturating_add(buf_len).div_ceil(PAGE_SIZE as u64);
	let mut buf_off = 0;
	for page_off in start..end {
		let page = node.node_ops.read_page(node, page_off)?;
		let inner_off = off as usize % PAGE_SIZE;
		let len = min(size - off, (PAGE_SIZE - inner_off) as u64) as usize;
		let len = unsafe {
			let page_ptr = page.virt_addr().as_ptr::<u8>().add(inner_off);
			buf.copy_to_user_raw(buf_off, page_ptr, len)?
		};
		buf_off += len;
		off += len as u64;
	}
	Ok(buf_off)
}

/// Generic implementation for [`FileOps::write`] on regular files.
///
/// **Note**: `file` **must** have an associated [`Node`], otherwise the function panics.
pub fn generic_file_write(file: &File, mut off: u64, buf: UserSlice<u8>) -> EResult<usize> {
	let node = file.node().unwrap();
	let size = file.stat()?.size;
	if unlikely(off > size) {
		return Err(errno!(EINVAL));
	}
	// Extend the file if necessary
	let end = off + buf.len() as u64; // FIXME: overflow
	if end > size {
		file.ops.truncate(file, end)?;
	}
	let start = off / PAGE_SIZE as u64;
	let end = off
		.saturating_add(buf.len() as u64)
		.div_ceil(PAGE_SIZE as u64);
	let mut buf_off = 0;
	for page_off in start..end {
		let page = node.node_ops.read_page(node, page_off)?;
		let inner_off = off as usize % PAGE_SIZE;
		let len = unsafe {
			let page_ptr = page.virt_addr().as_ptr::<u8>().add(inner_off);
			buf.copy_from_user_raw(buf_off, page_ptr, PAGE_SIZE - inner_off)?
		};
		page.mark_dirty();
		buf_off += len;
		off += len as u64;
	}
	Ok(buf_off)
}

#[derive(Debug)]
struct DummyOps;

impl NodeOps for DummyOps {}

impl FileOps for DummyOps {}

/// Filesystem operations.
pub trait FilesystemOps: Any + Debug {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &[u8];
	/// Returns statistics about the filesystem.
	fn get_stat(&self) -> EResult<Statfs>;

	/// Returns the root node.
	///
	/// If the node does not exist, the function returns [`errno::ENOENT`].
	fn root(&self, fs: &Arc<Filesystem>) -> EResult<Arc<Node>>;

	/// Creates a node on the filesystem.
	fn create_node(&self, fs: &Arc<Filesystem>, stat: Stat) -> EResult<Arc<Node>>;

	/// Removes `node` from the filesystem.
	///
	/// This function should be called only when no link to the node remain.
	fn destroy_node(&self, node: &Node) -> EResult<()>;

	/// Synchronizes the filesystem to its backing storage.
	///
	/// The default implementation of this function does nothing.
	fn sync_fs(&self) -> EResult<()> {
		Ok(())
	}
}

/// Downcasts the given `fs` into `F`.
///
/// If the filesystem type do not match, the function panics.
pub fn downcast_fs<F: FilesystemOps>(fs: &dyn FilesystemOps) -> &F {
	(fs as &dyn Any).downcast_ref().unwrap()
}

struct NodeWrapper(Arc<Node>);

impl Borrow<INode> for NodeWrapper {
	fn borrow(&self) -> &INode {
		&self.0.inode
	}
}

impl Eq for NodeWrapper {}

impl PartialEq for NodeWrapper {
	fn eq(&self, other: &Self) -> bool {
		self.0.inode == other.0.inode
	}
}

impl Hash for NodeWrapper {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.inode.hash(state)
	}
}

impl fmt::Debug for NodeWrapper {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.0, f)
	}
}

/// A filesystem.
#[derive(Debug)]
pub struct Filesystem {
	/// Device number
	pub dev: u64,
	/// Filesystem operations
	pub ops: Box<dyn FilesystemOps>,

	/// Cached [`Node`]s, to avoid duplications when several entries point to the same node
	nodes: Mutex<HashSet<NodeWrapper>>,
	/// Active buffers on the filesystem
	buffers: Mutex<HashMap<INode, Arc<dyn FileOps>>>,
}

impl Filesystem {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `dev` is the device number
	/// - `ops` is the handle for operations on the filesystem
	pub fn new(dev: u64, ops: Box<dyn FilesystemOps>) -> AllocResult<Arc<Self>> {
		Arc::new(Self {
			dev,
			ops,

			nodes: Default::default(),
			buffers: Default::default(),
		})
	}

	/// Get the buffer associated with the ID `inode` from cache. If not present, initialize it
	/// with `init`.
	pub fn buffer_get_or_insert<F: FileOps, Init: FnOnce() -> AllocResult<F>>(
		&self,
		inode: INode,
		init: Init,
	) -> AllocResult<Arc<dyn FileOps>> {
		let mut buffers = self.buffers.lock();
		if let Some(buf) = buffers.get(&inode) {
			return Ok(buf.clone());
		}
		let buf = Arc::new(init()?)?;
		buffers.insert(inode, buf.clone())?;
		Ok(buf)
	}

	/// Inserts a node in cache. If already present, the previous entry is dropped.
	pub fn node_insert(&self, node: Arc<Node>) -> EResult<()> {
		self.nodes.lock().insert(NodeWrapper(node))?;
		Ok(())
	}

	/// Returns the node with ID `inode` from the cache, or if not in cache, initializes it with
	/// `init` and inserts it.
	pub fn node_get_or_insert<F: FnOnce() -> EResult<Arc<Node>>>(
		&self,
		inode: INode,
		init: F,
	) -> EResult<Arc<Node>> {
		let mut nodes = self.nodes.lock();
		match nodes.get(&inode) {
			// Cache hit
			Some(node) => Ok(node.0.clone()),
			// Cache miss, create instance and insert
			None => {
				let node = init()?;
				nodes.insert(NodeWrapper(node.clone()))?;
				Ok(node)
			}
		}
	}

	/// Removes the node with ID `inode` from the cache.
	pub fn node_remove(&self, inode: INode) {
		self.nodes.lock().remove(&inode);
	}

	/// Synchronizes the whole filesystem to disk.
	pub fn sync(&self) -> EResult<()> {
		// Synchronize all nodes to disk
		let nodes = self.nodes.lock();
		for node in nodes.iter() {
			node.0.sync(true)?;
		}
		// Synchronize filesystem structures
		self.ops.sync_fs()
	}
}

impl Drop for Filesystem {
	fn drop(&mut self) {
		// TODO warning on error?
		let _ = self.sync();
	}
}

/// A filesystem type.
pub trait FilesystemType {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &'static [u8];

	/// Tells whether the given IO interface has the current filesystem.
	///
	/// `dev` is the device containing the potential filesystem
	fn detect(&self, dev: &Arc<BlkDev>) -> EResult<bool>;

	/// Creates a new instance of the filesystem to mount it.
	///
	/// Arguments:
	/// - `dev` is the mounted device
	/// - `mountpath` is the path on which the filesystem is mounted
	/// - `readonly` tells whether the filesystem is mounted in read-only
	fn load_filesystem(
		&self,
		dev: Option<Arc<BlkDev>>,
		mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<Filesystem>>;
}

/// The list of filesystem types.
static FS_TYPES: Mutex<HashMap<String, Arc<dyn FilesystemType>>> = Mutex::new(HashMap::new());

/// Registers a new filesystem type.
pub fn register<T: 'static + FilesystemType>(fs_type: T) -> EResult<()> {
	let name = String::try_from(fs_type.get_name())?;
	FS_TYPES.lock().insert(name, Arc::new(fs_type)?)?;
	Ok(())
}

/// Unregisters the filesystem type with the given name.
///
/// If the filesystem type doesn't exist, the function does nothing.
pub fn unregister(name: &[u8]) {
	FS_TYPES.lock().remove(name);
}

/// Returns the filesystem type with name `name`.
pub fn get_type(name: &[u8]) -> Option<Arc<dyn FilesystemType>> {
	FS_TYPES.lock().get(name).cloned()
}

/// Detects the filesystem type on device
pub fn detect(dev: &Arc<BlkDev>) -> EResult<Arc<dyn FilesystemType>> {
	let fs_types = FS_TYPES.lock();
	for (_, fs_type) in fs_types.iter() {
		if fs_type.detect(dev)? {
			return Ok(fs_type.clone());
		}
	}
	Err(errno!(ENODEV))
}

/// Registers the filesystems that are implemented inside the kernel itself.
///
/// This function must be called only once, at initialization.
pub fn register_defaults() -> EResult<()> {
	register(ext2::Ext2FsType)?;
	register(tmp::TmpFsType)?;
	register(proc::ProcFsType)?;
	// TODO sysfs
	Ok(())
}
