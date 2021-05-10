//! A mount point is a directory in which a filesystem is mounted.

use crate::device::Device;
use crate::device::DeviceType;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::filesystem::Filesystem;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::ptr::SharedPtr;
use super::filesystem;
use super::path::Path;

/// Structure representing a mount point.
pub struct MountPoint {
	/// The device type.
	device_type: DeviceType,
	/// The minor number of the device.
	minor: u32,
	/// The major number of the device.
	major: u32,

	/// The path to the mount directory.
	path: Path,

	/// An instance of the filesystem associated with the mountpoint.
	filesystem: Box<dyn Filesystem>,
}

impl MountPoint {
	/// Creates a new instance.
	/// `device_type` is the type of the device.
	/// `major` is the major number of the device.
	/// `minor` is the minor number of the device.
	/// `path` is the path on which the filesystem is to be mounted.
	pub fn new(device_type: DeviceType, major: u32, minor: u32, path: Path)
		-> Result<Self, Errno> {
		let mut device = device::get_device(device_type, major, minor).ok_or(errno::ENODEV)?;
		let fs_type = filesystem::detect(device.as_mut())?;
		let filesystem = fs_type.new_filesystem(device.as_mut().get_handle())?;

		Ok(Self {
			device_type: device_type,
			major: major,
			minor: minor,

			path: path,

			filesystem: filesystem,
		})
	}

	/// Returns the type of the mounted device.
	pub fn get_device_type(&self) -> DeviceType {
		self.device_type
	}

	/// Returns the major number of the mounted device.
	pub fn get_major(&self) -> u32 {
		self.major
	}

	/// Returns the minor number of the mounted device.
	pub fn get_minor(&self) -> u32 {
		self.minor
	}

	/// Returns a reference to the mounted device.
	pub fn get_device(&self) -> Option<SharedPtr<Device>> {
		device::get_device(self.device_type, self.major, self.minor)
	}

	/// Returns a reference to the path where the filesystem is mounted.
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Returns a mutable reference to the filesystem associated with the device.
	pub fn get_filesystem(&mut self) -> &mut dyn Filesystem {
		self.filesystem.as_mut()
	}
}

/// The list of mountpoints.
static mut MOUNT_POINTS: Mutex<Vec<SharedPtr<MountPoint>>> = Mutex::new(Vec::new());

/// Registers a new mountpoint `mountpoint`. If a mountpoint is already present at the same path,
/// the function fails.
pub fn register_mountpoint(mountpoint: MountPoint) -> Result<SharedPtr<MountPoint>, Errno> {
	let mutex = unsafe { // Safe because using Mutex
		&mut MOUNT_POINTS
	};
	let mut guard = MutexGuard::new(mutex);
	let container = guard.get_mut();
	let shared_ptr = SharedPtr::new(mountpoint)?;
	container.push(shared_ptr.clone())?;
	Ok(shared_ptr)
}
