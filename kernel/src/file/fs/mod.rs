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
pub mod procfs;
pub mod tmp;

use super::{path::PathBuf, File};
use crate::file::INode;
use core::{any::Any, fmt::Debug};
use utils::{
	collections::{hashmap::HashMap, string::String},
	errno,
	errno::EResult,
	io::IO,
	lock::Mutex,
	ptr::arc::Arc,
};

/// This structure is used in the f_fsid field of statfs. It is currently
/// unused.
#[repr(C)]
#[derive(Debug, Default)]
struct Fsid {
	/// Unused.
	_val: [i32; 2],
}

/// Structure storing statistics about a filesystem.
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

/// Trait representing a filesystem.
pub trait Filesystem: Any + Debug {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &[u8];
	/// Tells whether the filesystem is mounted in read-only.
	fn is_readonly(&self) -> bool;
	/// Tells the kernel can cache the filesystem's files in memory.
	fn use_cache(&self) -> bool;
	/// Returns the root inode of the filesystem.
	fn get_root_inode(&self) -> INode;
	/// Returns statistics about the filesystem.
	fn get_stat(&self, io: &mut dyn IO) -> EResult<Statfs>;

	/// Returns the inode of the file with name `name`, located in the directory
	/// with inode `parent`.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `parent` is the inode's parent. If `None`, the function uses the root of
	/// the filesystem.
	/// - `name` is the name of the file.
	///
	/// If the parent is not a directory, the function returns an error.
	fn get_inode(&mut self, io: &mut dyn IO, parent: Option<INode>, name: &[u8])
		-> EResult<INode>;

	/// Loads the node at inode `inode`.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `inode` is the node's ID.
	fn load_file(&mut self, io: &mut dyn IO, inode: INode) -> EResult<File>;

	/// Adds a file to the filesystem.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `parent_inode` is the parent file's inode.
	/// - `name` is the name of the file.
	/// - `node` is the node to add.
	///
	/// On success, the function returns the updated `node`.
	fn add_file(
		&mut self,
		io: &mut dyn IO,
		parent_inode: INode,
		name: &[u8],
		node: File,
	) -> EResult<File>;

	/// Adds a hard link to the filesystem.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `parent_inode` is the parent file's inode.
	/// - `name` is the name of the link.
	/// - `inode` is the inode the link points to.
	///
	/// If this feature is not supported by the filesystem, the function returns
	/// an error.
	fn add_link(
		&mut self,
		io: &mut dyn IO,
		parent_inode: INode,
		name: &[u8],
		inode: INode,
	) -> EResult<()>;

	/// Updates the given node.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `file` the file structure containing the new values for the inode.
	fn update_inode(&mut self, io: &mut dyn IO, file: &File) -> EResult<()>;

	/// Removes a file from the filesystem. If the links count of the inode
	/// reaches zero, the node is also removed.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `parent_inode` is the parent file's inode.
	/// - `name` is the file's name.
	///
	/// The function returns the number of hard links left on the node and the node's ID.
	fn remove_file(
		&mut self,
		io: &mut dyn IO,
		parent_inode: INode,
		name: &[u8],
	) -> EResult<(u16, INode)>;

	/// Reads from the node with ID `inode` into the buffer `buf`.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `inode` is the file's inode.
	/// - `off` is the offset from which the data will be read from the node's data.
	/// - `buf` is the buffer in which the data is to be written. The length of the buffer is the
	/// number of bytes to read.
	///
	/// This function is relevant for the following file types:
	/// - `Regular`: Reads the content of the file
	/// - `Link`: Reads the path the link points to
	///
	/// The function returns the number of bytes read.
	fn read_node(
		&mut self,
		io: &mut dyn IO,
		inode: INode,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64>;

	/// Writes to the given inode `inode` from the buffer `buf`.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `inode` is the file's inode.
	/// - `off` is the offset at which the data will be written in the node's data.
	/// - `buf` is the buffer in which the data is to be read from. The length of the buffer is the
	/// number of bytes to write.
	///
	/// This function is relevant for the following file types:
	/// - `Regular`: Writes the content of the file
	/// - `Link`: Writes the path the link points to. The path is truncated to `off` before writing
	fn write_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &[u8]) -> EResult<()>;
}

/// Trait representing a filesystem type.
pub trait FilesystemType {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &'static [u8];

	/// Tells whether the given IO interface has the current filesystem.
	///
	/// `io` is the IO interface.
	fn detect(&self, io: &mut dyn IO) -> EResult<bool>;

	/// Creates a new instance of the filesystem to mount it.
	///
	/// Arguments:
	/// - `io` is the IO interface.
	/// - `mountpath` is the path on which the filesystem is mounted.
	/// - `readonly` tells whether the filesystem is mounted in read-only.
	fn load_filesystem(
		&self,
		io: &mut dyn IO,
		mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<Mutex<dyn Filesystem>>>;
}

/// The list of filesystem types.
static FS_TYPES: Mutex<HashMap<String, Arc<dyn FilesystemType>>> = Mutex::new(HashMap::new());

/// Registers a new filesystem type.
pub fn register<T: 'static + FilesystemType>(fs_type: T) -> EResult<()> {
	let name = String::try_from(fs_type.get_name())?;
	let mut fs_types = FS_TYPES.lock();
	fs_types.insert(name, Arc::new(fs_type)?)?;
	Ok(())
}

/// Unregisters the filesystem type with the given name.
///
/// If the filesystem type doesn't exist, the function does nothing.
pub fn unregister(name: &[u8]) {
	let mut fs_types = FS_TYPES.lock();
	fs_types.remove(name);
}

/// Returns the filesystem type with name `name`.
pub fn get_type(name: &[u8]) -> Option<Arc<dyn FilesystemType>> {
	let fs_types = FS_TYPES.lock();
	fs_types.get(name).cloned()
}

/// Detects the filesystem type on the given IO interface `io`.
pub fn detect(io: &mut dyn IO) -> EResult<Arc<dyn FilesystemType>> {
	let fs_types = FS_TYPES.lock();
	for (_, fs_type) in fs_types.iter() {
		if fs_type.detect(io)? {
			return Ok(fs_type.clone());
		}
	}
	Err(errno!(ENODEV))
}

/// Registers the filesystems that are implemented inside the kernel itself.
///
/// This function must be called only once, at initialization.
pub fn register_defaults() -> EResult<()> {
	register(ext2::Ext2FsType {})?;
	register(tmp::TmpFsType {})?;
	register(procfs::ProcFsType {})?;
	// TODO sysfs
	Ok(())
}
