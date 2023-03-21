//! A mount point is a directory in which a filesystem is mounted.

use core::cmp::max;
use core::fmt;
use crate::device::DeviceID;
use crate::device::DeviceType;
use crate::device;
use crate::errno::Errno;
use crate::util::FailableClone;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::io::DummyIO;
use crate::util::io::IO;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;
use super::FileContent;
use super::fs::Filesystem;
use super::fs::FilesystemType;
use super::fs;
use super::path::Path;
use super::vfs;

/// Permits mandatory locking on files.
const FLAG_MANDLOCK: u32 = 0b000000000001;
/// Do not update file (all kinds) access timestamps on the filesystem.
const FLAG_NOATIME: u32 = 0b000000000010;
/// Do not allows access to device files on the filesystem.
const FLAG_NODEV: u32 = 0b000000000100;
/// Do not update directory access timestamps on the filesystem.
const FLAG_NODIRATIME: u32 = 0b000000001000;
/// Do not allow files on the filesystem to be executed.
const FLAG_NOEXEC: u32 = 0b000000010000;
/// Ignore setuid and setgid flags on the filesystem.
const FLAG_NOSUID: u32 = 0b000000100000;
/// Mounts the filesystem in read-only.
const FLAG_RDONLY: u32 = 0b000001000000;
/// TODO doc
const FLAG_REC: u32 = 0b000010000000;
/// Update atime only if less than or equal to mtime or ctime.
const FLAG_RELATIME: u32 = 0b000100000000;
/// Suppresses certain warning messages in the kernel logs.
const FLAG_SILENT: u32 = 0b001000000000;
/// Always update the last access time when files on this filesystem are
/// accessed. Overrides NOATIME and RELATIME.
const FLAG_STRICTATIME: u32 = 0b010000000000;
/// Makes writes on this filesystem synchronous.
const FLAG_SYNCHRONOUS: u32 = 0b100000000000;

// TODO When removing a mountpoint, return an error if another mountpoint is
// present in a subdir

/// Enumeration of mount sources.
#[derive(Eq, Hash, PartialEq)]
pub enum MountSource {
	/// The mountpoint is mounted from a device.
	Device {
		/// The device type.
		dev_type: DeviceType,

		/// The major number.
		major: u32,
		/// The minor number.
		minor: u32,
	},

	/// The mountpoint is bound to a virtual filesystem and thus isn't
	/// associated with any device. The string value is the name of the source.
	NoDev(String),
}

impl MountSource {
	/// Creates a mount source from a dummy string.
	///
	/// The string `string` might be either a kernfs name, a relative path or an
	/// absolute path.
	///
	/// `cwd` is the current working directory.
	pub fn from_str(string: &[u8], cwd: Path) -> Result<Self, Errno> {
		let path = Path::from_str(string, true)?;
		let path = cwd.concat(&path)?;
		let result = {
			let vfs_mutex = vfs::get();
			let mut vfs = vfs_mutex.lock();
			let vfs = vfs.as_mut().unwrap();

			vfs.get_file_from_path(&path, 0, 0, true)
		};

		match result {
			Ok(file_mutex) => {
				let file = file_mutex.lock();

				match file.get_content() {
					FileContent::BlockDevice { major, minor } => Ok(Self::Device {
						dev_type: DeviceType::Block,

						major: *major,
						minor: *minor,
					}),

					FileContent::CharDevice { major, minor } => Ok(Self::Device {
						dev_type: DeviceType::Char,

						major: *major,
						minor: *minor,
					}),

					_ => Err(errno!(EINVAL)),
				}
			},

			Err(err) if err == errno!(ENOENT) => Ok(Self::NoDev(String::try_from(string)?)),

			Err(err) => Err(err),
		}
	}

	/// Returns the IO interface for the mount source.
	pub fn get_io(&self) -> Result<SharedPtr<dyn IO>, Errno> {
		match self {
			Self::Device {
				dev_type,

				major,
				minor,
			} => {
				let dev = device::get(&DeviceID {
					type_: *dev_type,
					major: *major,
					minor: *minor,
				}).ok_or_else(|| errno!(ENODEV))?;
				Ok(dev as _)
			}

			Self::NoDev(_) => Ok(SharedPtr::new(DummyIO {})? as _),
		}
	}
}

impl FailableClone for MountSource {
	fn failable_clone(&self) -> Result<Self, Errno> {
		Ok(match self {
			Self::Device {
				dev_type,
				major,
				minor,
			} => Self::Device {
				dev_type: *dev_type,
				major: *major,
				minor: *minor,
			},

			Self::NoDev(name) => Self::NoDev(name.failable_clone()?),
		})
	}
}

impl fmt::Display for MountSource {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		match self {
			Self::Device {
				dev_type,

				major,
				minor,
			} => write!(fmt, "{}.{}.{}", dev_type, major, minor),

			Self::NoDev(name) => write!(fmt, "{}", name),
		}
	}
}

/// Structure wrapping a loaded filesystem.
struct LoadedFS {
	/// The number of mountpoints using the filesystem.
	ref_count: usize,

	/// The filesystem.
	fs: SharedPtr<dyn Filesystem>,
}

/// The list of loaded filesystems associated with their respective sources.
static FILESYSTEMS: Mutex<HashMap<MountSource, LoadedFS>> = Mutex::new(HashMap::new());

/// Loads a filesystem.
///
/// Arguments:
/// - `source` is the source of the mountpoint.
/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it
/// automaticaly.
/// - `path` is the path to the directory on which the filesystem is mounted.
/// - `readonly` tells whether the filesystem is mount in readonly.
///
/// On success, the function returns the loaded filesystem.
fn load_fs(
	source: MountSource,
	fs_type: Option<SharedPtr<dyn FilesystemType>>,
	path: Path,
	readonly: bool,
) -> Result<SharedPtr<dyn Filesystem>, Errno> {
	// Getting the I/O interface
	let io_mutex = source.get_io()?;
	let mut io = io_mutex.lock();

	// Getting the filesystem type
	let fs_type_mutex = match fs_type {
		Some(fs_type) => fs_type,

		None => match source {
			MountSource::NoDev(ref name) => fs::get_fs(name).ok_or_else(|| errno!(ENODEV))?,
			_ => fs::detect(&mut *io)?,
		},
	};
	let fs_type = fs_type_mutex.lock();

	let fs = fs_type.load_filesystem(&mut *io, path, readonly)?;

	// Inserting new filesystem into filesystems list
	let mut container = FILESYSTEMS.lock();
	container.insert(
		source,
		LoadedFS {
			ref_count: 1,

			fs: fs.clone(),
		},
	)?;

	Ok(fs)
}

/// Returns the loaded filesystem with the given source `source`.
/// `take` tells whether the function increments the references count.
/// If the filesystem isn't loaded, the function returns `None`.
fn get_fs_(source: &MountSource, take: bool) -> Option<SharedPtr<dyn Filesystem>> {
	let mut container = FILESYSTEMS.lock();

	let fs = container.get_mut(source)?;
	if take {
		fs.ref_count += 1;
	}

	Some(fs.fs.clone())
}

/// Returns the loaded filesystem with the given source `source`.
/// If the filesystem isn't loaded, the function returns `None`.
pub fn get_fs(source: &MountSource) -> Option<SharedPtr<dyn Filesystem>> {
	get_fs_(source, false)
}

/// Drops a reference to the filesystem with the given source `source`.
/// If no reference on the filesystem is left, the function unloads it.
/// If the filesystem doesn't exist, the function does nothing.
fn drop_fs(source: &MountSource) {
	let mut container = FILESYSTEMS.lock();

	if let Some(fs) = container.get_mut(source) {
		fs.ref_count -= 1;

		// If no reference left, drop
		if fs.ref_count <= 0 {
			container.remove(source);
		}
	}
}

/// Structure representing a mount point.
pub struct MountPoint {
	/// The ID of the mountpoint.
	id: u32,

	/// Mount flags.
	flags: u32,
	/// The path to the mount directory.
	path: Path,

	/// The source of the mountpoint.
	source: MountSource,
	/// The filesystem associated with the mountpoint.
	fs: SharedPtr<dyn Filesystem>,
	/// The name of the filesystem's type.
	fs_type_name: String,
}

impl MountPoint {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `id` is the ID of the mountpoint.
	/// - `source` is the source of the mountpoint.
	/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it
	/// automaticaly.
	/// - `flags` are the mount flags.
	/// - `path` is the path on which the filesystem is to be mounted.
	fn new(
		id: u32,
		source: MountSource,
		fs_type: Option<SharedPtr<dyn FilesystemType>>,
		flags: u32,
		path: Path,
	) -> Result<Self, Errno> {
		// Tells whether the filesystem will be mounted in read-only
		let readonly = flags & FLAG_RDONLY != 0;

		let fs_mutex = match get_fs_(&source, true) {
			// Filesystem exists, do nothing
			Some(fs) => fs,

			// Filesystem doesn't exist, load it
			None => load_fs(
				source.failable_clone()?,
				fs_type,
				path.failable_clone()?,
				readonly,
			)?,
		};

		// TODO Increment number of references to the filesystem

		let fs_type_name = {
			let fs = fs_mutex.lock();
			String::try_from(fs.get_name())?
		};

		Ok(Self {
			id,

			flags,
			path,

			source,
			fs: fs_mutex,
			fs_type_name,
		})
	}

	/// Returns the ID of the mountpoint.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns the mountpoint's flags.
	pub fn get_flags(&self) -> u32 {
		self.flags
	}

	/// Tells whether the mountpoint's is mounted in read-only.
	pub fn is_readonly(&self) -> bool {
		self.flags & FLAG_RDONLY != 0
	}

	/// Returns a reference to the path where the filesystem is mounted.
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Returns the source of the mountpoint.
	pub fn get_source(&self) -> &MountSource {
		&self.source
	}

	/// Returns a mutable reference to the filesystem associated with the
	/// mountpoint.
	pub fn get_filesystem(&self) -> SharedPtr<dyn Filesystem> {
		self.fs.clone()
	}

	/// Returns the name of the filesystem's type.
	pub fn get_filesystem_type(&self) -> &String {
		&self.fs_type_name
	}
}

impl Drop for MountPoint {
	fn drop(&mut self) {
		drop_fs(&self.source);
	}
}

/// The list of mountpoints with their respective ID.
pub static MOUNT_POINTS: Mutex<HashMap<u32, SharedPtr<MountPoint>>> = Mutex::new(HashMap::new());
/// A map from mountpoint paths to mountpoint IDs.
pub static PATH_TO_ID: Mutex<HashMap<Path, u32>> = Mutex::new(HashMap::new());

/// Creates a new mountpoint.
///
/// If a mountpoint is already present at the same path, the function fails.
///
/// Arguments:
/// - `source` is the source of the mountpoint.
/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it automaticaly.
/// - `flags` are the mount flags.
/// - `path` is the path on which the filesystem is to be mounted.
pub fn create(
	source: MountSource,
	fs_type: Option<SharedPtr<dyn FilesystemType>>,
	flags: u32,
	path: Path,
) -> Result<SharedPtr<MountPoint>, Errno> {
	// TODO clean
	// PATH_TO_ID is locked first and during the whole function to prevent a race condition between
	// the locks of MOUNT_POINTS
	let mut path_to_id = PATH_TO_ID.lock();

	// TODO clean
	// ID allocation
	let id = {
		let mut id = 0;

		for (i, _) in MOUNT_POINTS.lock().iter() {
			id = max(*i, id);
		}

		id + 1
	};

	let mountpoint = SharedPtr::new(MountPoint::new(
		id,
		source,
		fs_type,
		flags,
		path.failable_clone()?
	)?)?;

	// Insertion
	{
		let mut mount_points = MOUNT_POINTS.lock();

		mount_points.insert(id, mountpoint.clone())?;
		if let Err(e) = path_to_id.insert(path, id) {
			mount_points.remove(&id);
			return Err(e);
		}
	}

	Ok(mountpoint)
}

/// Removes the mountpoint at the given path `path`.
///
/// Data is sychronized to the associated storage device, if any, before removing the mountpoint.
///
/// If the mountpoint doesn't exist, the function returns `EINVAL`.
///
/// If the mountpoint is busy, the function returns `EBUSY`.
pub fn remove(path: &Path) -> Result<(), Errno> {
	let mut path_to_id = PATH_TO_ID.lock();
	let mut mount_points = MOUNT_POINTS.lock();

	let id = path_to_id.get(path).ok_or(errno!(EINVAL))?.clone();
	let _mountpoint = mount_points.get(&id).ok_or(errno!(EINVAL))?;

	// TODO Check if busy (EBUSY)
	// TODO Check if another mount point is present in a subdirectory (EBUSY)

	// TODO sync fs

	path_to_id.remove(path);
	mount_points.remove(&id);

	Ok(())
}

/// Returns the deepest mountpoint in the path `path`.
///
/// If no mountpoint is in the path, the function returns `None`.
pub fn get_deepest(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let container = MOUNT_POINTS.lock();

	let mut max: Option<SharedPtr<MountPoint>> = None;
	for (_, mp) in container.iter() {
		let mp_guard = mp.lock();
		let mount_path = mp_guard.get_path();

		if let Some(max) = max.as_mut() {
			let max = max.lock();
			let max_path = max.get_path();

			if max_path.get_elements_count() >= mount_path.get_elements_count() {
				continue;
			}
		}

		if path.begins_with(mount_path) {
			max = Some(mp.clone());
		}
	}

	max
}

/// Returns the mountpoint with id `id`.
///
/// If it doesn't exist, the function returns `None`.
pub fn from_id(id: u32) -> Option<SharedPtr<MountPoint>> {
	let container = MOUNT_POINTS.lock();

	for (mp_id, mp) in container.iter() {
		if *mp_id == id {
			return Some(mp.clone());
		}
	}

	None
}

/// Returns the mountpoint with path `path`.
///
/// If it doesn't exist, the function returns `None`.
pub fn from_path(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let container = MOUNT_POINTS.lock();

	for (_, mp) in container.iter() {
		let mp_guard = mp.lock();
		let mountpoint_path = mp_guard.get_path();

		if mountpoint_path == path {
			return Some(mp.clone());
		}
	}

	None
}
