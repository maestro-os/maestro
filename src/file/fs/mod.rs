//! A filesystem is the representation of the file hierarchy on a storage device.

pub mod ext2;

use crate::device::Device;
use crate::device::DeviceHandle;
use crate::errno::Errno;
use crate::errno;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::lock::mutex::TMutex;
use crate::util::ptr::SharedPtr;
use super::File;
use super::INode;
use super::path::Path;

/// Trait representing a filesystem.
pub trait Filesystem {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &str;

	/// Tells whether the filesystem is mounted in read-only.
	fn is_readonly(&self) -> bool;

	/// Returns the inode of the file at path `path`.
	/// `io` is the I/O interface.
	/// `path` is the file's path.
	/// The path must be absolute relative the filesystem's root directory and must not contain
	/// any `.` or `..` component.
	fn get_inode(&mut self, io: &mut dyn DeviceHandle, path: Path) -> Result<INode, Errno>;

	/// Loads the file at inode `inode`.
	/// `io` is the I/O interface.
	/// `inode` is the file's inode.
	/// `name` is the file's name.
	fn load_file(&mut self, io: &mut dyn DeviceHandle, inode: INode, name: String)
		-> Result<File, Errno>;

	/// Adds a file to the filesystem at inode `inode`.
	/// `io` is the I/O interface.
	/// `parent_inode` is the parent file's inode.
	/// `file` is the file to be added.
	fn add_file(&mut self, io: &mut dyn DeviceHandle, parent_inode: INode, file: File)
		-> Result<(), Errno>;

	/// Removes a file from the filesystem.
	/// `io` is the I/O interface.
	/// `parent_inode` is the parent file's inode.
	/// `name` is the file's name.
	fn remove_file(&mut self, io: &mut dyn DeviceHandle, parent_inode: INode, name: &String)
		-> Result<(), Errno>;

	/// Reads from the given inode `inode` into the buffer `buf`.
	fn read_node(&mut self, io: &mut dyn DeviceHandle, inode: INode, buf: &mut [u8])
		-> Result<(), Errno>;

	/// Writes to the given inode `inode` from the buffer `buf`.
	fn write_node(&mut self, io: &mut dyn DeviceHandle, inode: INode, buf: &[u8])
		-> Result<(), Errno>;
}

/// Trait representing a filesystem type.
pub trait FilesystemType {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &str;

	/// Tells whether the given device has the current filesystem.
	fn detect(&self, io: &mut dyn DeviceHandle) -> bool;

	/// Creates a new filesystem on the device and returns its instance.
	fn create_filesystem(&self, io: &mut dyn DeviceHandle) -> Result<Box<dyn Filesystem>, Errno>;

	/// Creates a new instance of the filesystem.
	fn load_filesystem(&self, io: &mut dyn DeviceHandle) -> Result<Box<dyn Filesystem>, Errno>;
}

/// The list of mountpoints.
static mut FILESYSTEMS: Mutex<Vec<SharedPtr<dyn FilesystemType>>> = Mutex::new(Vec::new());

/// Registers a new filesystem type `fs`.
pub fn register<T: 'static + FilesystemType>(fs_type: T) -> Result<(), Errno> {
	let mutex = unsafe { // Safe because using Mutex
		&mut FILESYSTEMS
	};
	let mut guard = MutexGuard::new(mutex);
	let container = guard.get_mut();
	container.push(SharedPtr::new(Mutex::new(fs_type))?)
}

// TODO Function to unregister a filesystem type

/// Detects the filesystem type on the given device `device`.
pub fn detect(device: &mut Device) -> Result<SharedPtr<dyn FilesystemType>, Errno> {
	let mutex = unsafe { // Safe because using Mutex
		&mut FILESYSTEMS
	};
	let mut guard = MutexGuard::new(mutex);
	let container = guard.get_mut();

	for i in 0..container.len() {
		let fs_type = &mut container[i];
		let fs_type_guard = fs_type.lock();

		if fs_type_guard.get().detect(device.get_handle()) {
			drop(fs_type_guard);
			return Ok(fs_type.clone()); // TODO Use a weak pointer?
		}
	}

	Err(errno::ENODEV)
}

/// Registers the filesystems that are implemented inside of the kernel itself.
/// This function must be called only once, at initialization.
pub fn register_defaults() -> Result<(), Errno> {
	register(ext2::Ext2FsType {})?;

	Ok(())
}
