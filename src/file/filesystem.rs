/// A filesystem is the representation of the file hierarchy on a storage device.

use crate::device::Device;
use crate::device::DeviceHandle;
use crate::errno::Errno;
use crate::errno;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::ptr::SharedPtr;
use super::File;
use super::INode;
use super::path::Path;

/// Trait representing a filesystem.
pub trait Filesystem {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &str;

	/// Tells whether the given device has the current filesystem.
	fn detect(&self, io: &mut dyn DeviceHandle) -> bool;

	/// Loads the file at path `path`.
	fn load_file(&mut self, io: &mut dyn DeviceHandle, path: Path) -> Result<File, Errno>;
	/// Reads from the given node `node` into the buffer `buf`.
	fn read_node(&mut self, io: &mut dyn DeviceHandle, node: INode, buf: &mut [u8])
		-> Result<(), Errno>;
	/// Writes to the given node `node` from the buffer `buf`.
	fn write_node(&mut self, io: &mut dyn DeviceHandle, node: INode, buf: &mut [u8])
		-> Result<(), Errno>;

	// TODO
}

/// The list of mountpoints.
static mut FILESYSTEMS: Mutex<Vec<SharedPtr<dyn Filesystem>>> = Mutex::new(Vec::new());

/// Registers a new filesystem `fs`.
pub fn register<T: 'static + Filesystem>(fs: T) -> Result<(), Errno> {
	let mutex = unsafe { // Safe because using Mutex
		&mut FILESYSTEMS
	};
	let mut guard = MutexGuard::new(mutex);
	let container = guard.get_mut();
	container.push(SharedPtr::new(fs)?)
}

// TODO Function to unregister a filesystem

/// Detects the filesystem on the given device `device`.
pub fn detect(device: &mut Device) -> Result<SharedPtr<dyn Filesystem>, Errno> {
	let mutex = unsafe { // Safe because using Mutex
		&mut FILESYSTEMS
	};
	let mut guard = MutexGuard::new(mutex);
	let container = guard.get_mut();

	for fs in container.iter() {
		if fs.detect(device.get_handle()) {
			return Ok(fs.clone()); // TODO Use a weak pointer?
		}
	}

	Err(errno::ENODEV)
}
