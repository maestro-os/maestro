//! A filesystem is the representation of the file hierarchy on a storage device.

pub mod ext2;
pub mod kernfs;
pub mod procfs;
pub mod tmp;

use core::any::Any;
use crate::errno::Errno;
use crate::errno;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::util::IO;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;
use super::File;
use super::path::Path;

/// Structure storing statistics about a filesystem.
#[repr(C)]
#[derive(Debug)]
pub struct Statfs {
	/// Type of filesystem.
	f_type: u32,
	/// Optimal transfer block size.
	f_bsize: u32,
	/// Total data blocks in filesystem.
	f_blocks: u32,
	/// Free blocks in filesystem.
	f_bfree: u32,
	/// Free blocks available to unprivileged user.
	f_bavail: u32,
	/// Total inodes in filesystem.
	f_files: u32,
	/// Free inodes in filesystem.
	f_ffree: u32,
	/// Filesystem ID.
	f_fsid: u64,
	/// Maximum length of filenames.
	f_namelen: u32,
	/// Fragment size.
	f_frsize: u32,
	/// Mount flags of filesystem.
	f_flags: u32,
}

/// Trait representing a filesystem.
pub trait Filesystem: Any {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &[u8];

	/// Tells whether the filesystem is mounted in read-only.
	fn is_readonly(&self) -> bool;
	/// Tells the kernel whether it must cache files.
	fn must_cache(&self) -> bool;

	/// Returns statistics about the filesystem.
	fn get_stat(&self, io: &mut dyn IO) -> Result<Statfs, Errno>;

	/// Returns the root inode of the filesystem.
	fn get_root_inode(&self, io: &mut dyn IO) -> Result<INode, Errno>;

	/// Returns the inode of the file with name `name`, located in the directory with inode
	/// `parent`.
	/// `io` is the IO interface.
	/// `parent` is the inode's parent. If none, the function uses the root of the filesystem.
	/// `name` is the name of the file.
	/// If the parent is not a directory, the function returns an error.
	fn get_inode(&mut self, io: &mut dyn IO, parent: Option<INode>, name: &String)
		-> Result<INode, Errno>;

	/// Loads the file at inode `inode`.
	/// `io` is the IO interface.
	/// `inode` is the file's inode.
	/// `name` is the file's name.
	fn load_file(&mut self, io: &mut dyn IO, inode: INode, name: String) -> Result<File, Errno>;

	/// Adds a file to the filesystem at inode `inode`.
	/// `io` is the IO interface.
	/// `parent_inode` is the parent file's inode.
	/// `name` is the name of the file.
	/// `uid` is the id of the owner user.
	/// `gid` is the id of the owner group.
	/// `mode` is the permission of the file.
	/// `content` is the content of the file. This value also determines the file type.
	/// On success, the function returns the newly created file.
	fn add_file(&mut self, io: &mut dyn IO, parent_inode: INode, name: String, uid: Uid,
		gid: Gid, mode: Mode, content: FileContent) -> Result<File, Errno>;

	/// Adds a hard link to the filesystem.
	/// If this feature is not supported by the filesystem, the function returns an error.
	/// `io` is the IO interface.
	/// `parent_inode` is the parent file's inode.
	/// `name` is the name of the link.
	/// `inode` is the inode the link points to.
	fn add_link(&mut self, io: &mut dyn IO, parent_inode: INode, name: &String, inode: INode)
		-> Result<(), Errno>;

	/// Updates the given inode.
	/// `io` is the IO interface.
	/// `file` the file structure containing the new values for the inode.
	fn update_inode(&mut self, io: &mut dyn IO, file: &File) -> Result<(), Errno>;

	/// Removes a file from the filesystem. If the links count of the inode reaches zero, the inode
	/// is also removed.
	/// `io` is the IO interface.
	/// `parent_inode` is the parent file's inode.
	/// `name` is the file's name.
	fn remove_file(&mut self, io: &mut dyn IO, parent_inode: INode, name: &String)
		-> Result<(), Errno>;

	/// Reads from the given inode `inode` into the buffer `buf`.
	/// `off` is the offset from which the data will be read from the node.
	/// The function returns a tuple containing:
	/// - The number of bytes read.
	/// - Whether the End Of File (EOF) has been reached.
	fn read_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &mut [u8])
		-> Result<(u64, bool), Errno>;

	/// Writes to the given inode `inode` from the buffer `buf`.
	/// `off` is the offset at which the data will be written in the node.
	fn write_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &[u8])
		-> Result<(), Errno>;
}

/// Trait representing a filesystem type.
pub trait FilesystemType {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &[u8];

	/// Tells whether the given IO interface has the current filesystem.
	/// `io` is the IO interface.
	fn detect(&self, io: &mut dyn IO) -> Result<bool, Errno>;

	/// Creates a new filesystem on the IO interface and returns its instance.
	/// `io` is the IO interface.
	/// `fs_id` is the ID of the loaded filesystem. This ID is only used by the kernel and not
	/// saved on the storage device.
	fn create_filesystem(&self, io: &mut dyn IO) -> Result<SharedPtr<dyn Filesystem>, Errno>;

	/// Creates a new instance of the filesystem to mount it.
	/// `io` is the IO interface.
	/// `mountpath` is the path on which the filesystem is mounted.
	/// `readonly` tells whether the filesystem is mounted in read-only.
	fn load_filesystem(&self, io: &mut dyn IO, mountpath: Path, readonly: bool)
		-> Result<SharedPtr<dyn Filesystem>, Errno>;
}

/// The list of filesystem types.
static FILESYSTEMS: Mutex<Vec<SharedPtr<dyn FilesystemType>>> = Mutex::new(Vec::new());

/// Registers a new filesystem type `fs`.
pub fn register<T: 'static + FilesystemType>(fs_type: T) -> Result<(), Errno> {
	let guard = FILESYSTEMS.lock();
	let container = guard.get_mut();
	container.push(SharedPtr::new(fs_type)?)
}

// TODO Function to unregister a filesystem type

// TODO Optimize
/// Returns the filesystem with name `name`.
pub fn get_fs(name: &[u8]) -> Option<SharedPtr<dyn FilesystemType>> {
	let guard = FILESYSTEMS.lock();
	let container = guard.get_mut();

	for i in 0..container.len() {
		let fs_type = &mut container[i];
		let fs_type_guard = fs_type.lock();

		if fs_type_guard.get().get_name() == name {
			drop(fs_type_guard);
			return Some(fs_type.clone());
		}
	}

	None
}

/// Detects the filesystem type on the given IO interface `io`.
pub fn detect(io: &mut dyn IO) -> Result<SharedPtr<dyn FilesystemType>, Errno> {
	let guard = FILESYSTEMS.lock();
	let container = guard.get_mut();

	for i in 0..container.len() {
		let fs_type = &mut container[i];
		let fs_type_guard = fs_type.lock();

		if fs_type_guard.get().detect(io)? {
			drop(fs_type_guard);
			return Ok(fs_type.clone()); // TODO Use a weak pointer?
		}
	}

	Err(errno!(ENODEV))
}

/// Registers the filesystems that are implemented inside of the kernel itself.
/// This function must be called only once, at initialization.
pub fn register_defaults() -> Result<(), Errno> {
	register(ext2::Ext2FsType {})?;
	register(tmp::TmpFsType {})?;
	register(procfs::ProcFsType {})?;
	// TODO sysfs

	Ok(())
}
