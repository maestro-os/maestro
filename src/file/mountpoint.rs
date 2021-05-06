/// A mount point is a directory in which a filesystem is mounted.

use crate::errno::Errno;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::ptr::SharedPtr;
use super::path::Path;

/// Structure representing a mount point.
pub struct MountPoint {
	/// The minor number of the device.
	minor: u32,
	/// The major number of the device.
	major: u32,

	/// The path to the mount directory.
	path: Path,

	// TODO Filesystem
}

impl MountPoint {
	/// Creates a new instance.
	/// `major` is the major number of the device.
	/// `minor` is the minor number of the device.
	/// `path` is the path on which the filesystem is to be mounted.
	pub fn new(major: u32, minor: u32, path: Path) -> Self {
		Self {
			major: major,
			minor: minor,

			path: path,
		}
	}

	/// Returns a reference to the path where the filesystem is mounted.
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	// TODO Function to get device
	// TODO Function to get filesystem
}

/// The list of mountpoints.
static mut MOUNT_POINTS: Mutex<Vec<SharedPtr<MountPoint>>> = Mutex::new(Vec::new());

/// Registers a new mountpoint `mountpoint`. If a mountpoint is already present at the same path,
/// the function fails.
pub fn register_mountpoint(mountpoint: MountPoint) -> Result<SharedPtr<MountPoint>, Errno> {
	let mutex = unsafe { // Safe becauuse using Mutex
		&mut MOUNT_POINTS
	};
	let mut guard = MutexGuard::new(mutex);
	let container = guard.get_mut();
	let shared_ptr = SharedPtr::new(mountpoint)?;
	container.push(shared_ptr.clone())?;
	Ok(shared_ptr)
}
