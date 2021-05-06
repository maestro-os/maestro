/// A mount point is a directory in which a filesystem is mounted.

use super::path::Path;

/// Structure representing a mount point.
pub struct MountPoint {
	/// The minor number of the device.
	minor: u32,
	/// The major number of the device.
	major: u32,

	/// The path to the mount directory.
	path: Path,
}

// TODO Implement MountPoint
