//! A mount point is a directory in which a filesystem is mounted.

use crate::device::Device;
use crate::device::DeviceType;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::fs::Filesystem;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::lock::mutex::TMutex;
use crate::util::ptr::SharedPtr;
use super::fs;
use super::path::Path;

// TODO rm
use crate::file::fs::FilesystemType;

/// TODO doc
const FLAG_MANDLOCK: u32    = 0b000000000001;
/// TODO doc
const FLAG_NOATIME: u32     = 0b000000000010;
/// TODO doc
const FLAG_NODEV: u32       = 0b000000000100;
/// TODO doc
const FLAG_NODIRATIME: u32  = 0b000000001000;
/// TODO doc
const FLAG_NOEXEC: u32      = 0b000000010000;
/// TODO doc
const FLAG_NOSUID: u32      = 0b000000100000;
/// TODO doc
const FLAG_RDONLY: u32      = 0b000001000000;
/// TODO doc
const FLAG_REC: u32         = 0b000010000000;
/// TODO doc
const FLAG_RELATIME: u32    = 0b000100000000;
/// TODO doc
const FLAG_SILENT: u32      = 0b001000000000;
/// TODO doc
const FLAG_STRICTATIME: u32 = 0b010000000000;
/// TODO doc
const FLAG_SYNCHRONOUS: u32 = 0b100000000000;

// TODO When removing a mountpoint, return an error if another mountpoint is present in a subdir

/// Structure representing a mount point.
pub struct MountPoint {
	/// The device type.
	device_type: DeviceType,
	/// The minor number of the device.
	minor: u32,
	/// The major number of the device.
	major: u32,

	/// Mount flags.
	flags: u32,
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
	/// `flags` are the mount flags.
	/// `path` is the path on which the filesystem is to be mounted.
	pub fn new(device_type: DeviceType, major: u32, minor: u32, flags: u32, path: Path)
		-> Result<Self, Errno> {
		let mut dev_ptr = device::get_device(device_type, major, minor).ok_or(errno::ENODEV)?;
		let mut dev_guard = dev_ptr.lock();
		let device = dev_guard.get_mut();

		// TODO rm
		let fs_type = fs::ext2::Ext2FsType {};
		fs_type.create_filesystem(device.get_handle())?;

		let mut fs_type_ptr = fs::detect(device)?;
		let fs_type_guard = fs_type_ptr.lock();
		let fs_type = fs_type_guard.get();
		let filesystem = fs_type.load_filesystem(device.get_handle())?;

		Ok(Self {
			device_type,
			minor,
			major,

			flags,
			path,

			filesystem,
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
	pub fn get_device(&self) -> SharedPtr<Device> {
		device::get_device(self.device_type, self.major, self.minor).unwrap()
	}

	/// Returns the mountpoint's flags.
	pub fn get_flags(&self) -> u32 {
		self.flags
	}

	/// Returns a reference to the path where the filesystem is mounted.
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Returns a mutable reference to the filesystem associated with the device.
	pub fn get_filesystem(&mut self) -> &mut dyn Filesystem {
		self.filesystem.as_mut()
	}

	/// Tells whether the mountpoint's filesystem is mounted in read-only.
	pub fn is_readonly(&self) -> bool {
		self.flags & FLAG_RDONLY != 0 || self.filesystem.is_readonly()
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
	let shared_ptr = SharedPtr::new(Mutex::new(mountpoint))?;
	container.push(shared_ptr.clone())?;
	Ok(shared_ptr)
}

/// Returns the deepest mountpoint in the path `path`. If no mountpoint is in the path, the
/// function returns None.
pub fn get_deepest(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let mutex = unsafe { // Safe because using Mutex
		&mut MOUNT_POINTS
	};
	let mut guard = MutexGuard::new(mutex);
	let container = guard.get_mut();

	let mut max: Option<SharedPtr<MountPoint>> = None;
	for i in 0..container.len() {
		let m = &mut container[i];
		let mount_path_guard = m.lock();
		let mount_path = mount_path_guard.get().get_path();

		if let Some(max) = max.as_mut() {
			let max_guard = max.lock();
            let max_path = max_guard.get().get_path();

			if max_path.get_elements_count() >= mount_path.get_elements_count() {
				continue;
			}
		}

		if path.begins_with(mount_path) {
			drop(mount_path_guard);
			max = Some(m.clone());
		}
	}

	max
}
