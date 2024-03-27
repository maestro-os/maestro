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

//! A mount point is a directory in which a filesystem is mounted.

use super::{
	fs,
	fs::{Filesystem, FilesystemType},
	path::{Path, PathBuf},
	vfs, FileLocation, FileType,
};
use crate::{
	device,
	device::{DeviceID, DeviceType},
	file::vfs::ResolutionSettings,
};
use core::fmt;
use utils::{
	collections::{hashmap::HashMap, string::String},
	errno,
	errno::{AllocResult, EResult},
	io::{DummyIO, IO},
	lock::Mutex,
	ptr::arc::Arc,
	TryClone,
};

/// Permits mandatory locking on files.
pub const FLAG_MANDLOCK: u32 = 0b000000000001;
/// Do not update file (all kinds) access timestamps on the filesystem.
pub const FLAG_NOATIME: u32 = 0b000000000010;
/// Do not allows access to device files on the filesystem.
pub const FLAG_NODEV: u32 = 0b000000000100;
/// Do not update directory access timestamps on the filesystem.
pub const FLAG_NODIRATIME: u32 = 0b000000001000;
/// Do not allow files on the filesystem to be executed.
pub const FLAG_NOEXEC: u32 = 0b000000010000;
/// Ignore setuid and setgid flags on the filesystem.
pub const FLAG_NOSUID: u32 = 0b000000100000;
/// Mounts the filesystem in read-only.
pub const FLAG_RDONLY: u32 = 0b000001000000;
/// TODO doc
pub const FLAG_REC: u32 = 0b000010000000;
/// Update atime only if less than or equal to mtime or ctime.
pub const FLAG_RELATIME: u32 = 0b000100000000;
/// Suppresses certain warning messages in the kernel logs.
pub const FLAG_SILENT: u32 = 0b001000000000;
/// Always update the last access time when files on this filesystem are
/// accessed. Overrides NOATIME and RELATIME.
pub const FLAG_STRICTATIME: u32 = 0b010000000000;
/// Makes writes on this filesystem synchronous.
pub const FLAG_SYNCHRONOUS: u32 = 0b100000000000;

// TODO When removing a mountpoint, return an error if another mountpoint is
// present in a subdir

/// Value specifying the device from which a filesystem is mounted.
#[derive(Debug, Eq, Hash, PartialEq)]
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
	/// associated with any device.
	///
	/// The string value is the name of the source.
	NoDev(String),
}

impl MountSource {
	/// Creates a mount source from a dummy string.
	///
	/// The string `string` might be either a kernfs name, a relative path or an
	/// absolute path.
	pub fn new(string: &[u8]) -> EResult<Self> {
		let path = Path::new(string)?;
		let result = vfs::get_file_from_path(path, &ResolutionSettings::kernel_follow());
		match result {
			Ok(file_mutex) => {
				let file = file_mutex.lock();
				match file.get_content() {
					FileType::BlockDevice => Ok(Self::Device {
						dev_type: DeviceType::Block,
						major: file.dev_major,
						minor: file.dev_minor,
					}),
					FileType::CharDevice => Ok(Self::Device {
						dev_type: DeviceType::Char,
						major: file.dev_major,
						minor: file.dev_minor,
					}),
					_ => Err(errno!(EINVAL)),
				}
			}
			Err(err) if err == errno!(ENOENT) => Ok(Self::NoDev(String::try_from(string)?)),
			Err(err) => Err(err),
		}
	}

	/// Returns the IO interface for the mount source.
	pub fn get_io(&self) -> EResult<Arc<Mutex<dyn IO>>> {
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
				})
				.ok_or_else(|| errno!(ENODEV))?;
				Ok(dev as _)
			}

			Self::NoDev(_) => Ok(Arc::new(Mutex::new(DummyIO {}))? as _),
		}
	}
}

impl TryClone for MountSource {
	fn try_clone(&self) -> AllocResult<Self> {
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

			Self::NoDev(name) => Self::NoDev(name.try_clone()?),
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
			} => write!(fmt, "dev({dev_type}:{major}:{minor})"),

			Self::NoDev(name) => write!(fmt, "{name}"),
		}
	}
}

/// Structure wrapping a loaded filesystem.
struct LoadedFS {
	/// The number of mountpoints using the filesystem.
	ref_count: usize,

	/// The filesystem.
	fs: Arc<Mutex<dyn Filesystem>>,
}

/// The list of loaded filesystems associated with their respective sources.
static FILESYSTEMS: Mutex<HashMap<MountSource, LoadedFS>> = Mutex::new(HashMap::new());

/// Loads a filesystem.
///
/// Arguments:
/// - `source` is the source of the mountpoint.
/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it
/// automatically.
/// - `path` is the path to the directory on which the filesystem is mounted.
/// - `readonly` tells whether the filesystem is mount in readonly.
///
/// On success, the function returns the loaded filesystem.
fn load_fs(
	source: MountSource,
	fs_type: Option<Arc<dyn FilesystemType>>,
	path: PathBuf,
	readonly: bool,
) -> EResult<Arc<Mutex<dyn Filesystem>>> {
	// Get the I/O interface
	let io_mutex = source.get_io()?;
	let mut io = io_mutex.lock();

	// Get the filesystem type
	let fs_type = match fs_type {
		Some(fs_type) => fs_type,
		None => match source {
			MountSource::NoDev(ref name) => fs::get_type(name).ok_or_else(|| errno!(ENODEV))?,
			_ => fs::detect(&mut *io)?,
		},
	};
	let fs = fs_type.load_filesystem(&mut *io, path, readonly)?;

	// Insert new filesystem into filesystems list
	let mut filesystems = FILESYSTEMS.lock();
	filesystems.insert(
		source,
		LoadedFS {
			ref_count: 1,

			fs: fs.clone(),
		},
	)?;

	Ok(fs)
}

/// Returns the loaded filesystem with the given source `source`.
///
/// `acquire` tells whether the function increments the references count.
///
/// If the filesystem isn't loaded, the function returns `None`.
fn get_fs_impl(source: &MountSource, acquire: bool) -> Option<Arc<Mutex<dyn Filesystem>>> {
	let mut filesystems = FILESYSTEMS.lock();
	let fs = filesystems.get_mut(source)?;
	// Increment the number of references if required
	if acquire {
		fs.ref_count += 1;
	}
	Some(fs.fs.clone())
}

/// Returns the loaded filesystem with the given source `source`.
///
/// If the filesystem isn't loaded, the function returns `None`.
pub fn get_fs(source: &MountSource) -> Option<Arc<Mutex<dyn Filesystem>>> {
	get_fs_impl(source, false)
}

/// A mount point, allowing to attach a filesystem to a directory on the VFS.
#[derive(Debug)]
pub struct MountPoint {
	/// The ID of the mountpoint.
	id: u32,
	/// Mount flags.
	flags: u32,

	/// The source of the mountpoint.
	source: MountSource,
	/// The filesystem associated with the mountpoint.
	fs: Arc<Mutex<dyn Filesystem>>,
	/// The name of the filesystem's type.
	fs_type_name: String,

	/// The path to the mount directory.
	target_path: PathBuf,
	/// The location of the mount directory on the parent filesystem.
	target_location: FileLocation,
}

impl MountPoint {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `id` is the ID of the mountpoint.
	/// - `source` is the source of the mountpoint.
	/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it
	/// automatically.
	/// - `flags` are the mount flags.
	/// - `target_path` is the path to the mount directory.
	/// - `target_location` is the location of the mount directory on the parent filesystem.
	fn new(
		id: u32,
		source: MountSource,
		fs_type: Option<Arc<dyn FilesystemType>>,
		flags: u32,
		target_path: PathBuf,
		target_location: FileLocation,
	) -> EResult<Self> {
		// Tells whether the filesystem will be mounted as read-only
		let readonly = flags & FLAG_RDONLY != 0;

		let fs = match get_fs_impl(&source, true) {
			// Filesystem exists, do nothing
			Some(fs) => fs,
			// Filesystem doesn't exist, load it
			None => load_fs(
				source.try_clone()?,
				fs_type,
				target_path.try_clone()?,
				readonly,
			)?,
		};
		let fs_type_name = String::try_from(fs.lock().get_name())?;

		Ok(Self {
			id,
			flags,

			source,
			fs,
			fs_type_name,

			target_path,
			target_location,
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

	/// Tells whether the mountpoint's is mounted as read-only.
	pub fn is_readonly(&self) -> bool {
		self.flags & FLAG_RDONLY != 0
	}

	/// Returns the source of the mountpoint.
	pub fn get_source(&self) -> &MountSource {
		&self.source
	}

	/// Returns the filesystem associated with the mountpoint.
	pub fn get_filesystem(&self) -> Arc<Mutex<dyn Filesystem>> {
		self.fs.clone()
	}

	/// Returns the name of the filesystem's type.
	pub fn get_filesystem_type(&self) -> &String {
		&self.fs_type_name
	}

	/// Returns a reference to the path where the filesystem is mounted.
	pub fn get_target_path(&self) -> &Path {
		&self.target_path
	}

	/// Returns a reference to the location of the mount directory on the parent filesystem.
	pub fn get_target_location(&self) -> &FileLocation {
		&self.target_location
	}
}

impl Drop for MountPoint {
	fn drop(&mut self) {
		// Decrement the number of references to the filesystem
		let mut filesystems = FILESYSTEMS.lock();
		if let Some(fs) = filesystems.get_mut(&self.source) {
			fs.ref_count -= 1;
			// If no reference left, drop
			if fs.ref_count == 0 {
				filesystems.remove(&self.source);
			}
		}
	}
}

/// The list of mountpoints with their respective ID.
pub static MOUNT_POINTS: Mutex<HashMap<u32, Arc<Mutex<MountPoint>>>> = Mutex::new(HashMap::new());
/// A map from mount locations to mountpoint IDs.
pub static LOC_TO_ID: Mutex<HashMap<FileLocation, u32>> = Mutex::new(HashMap::new());

/// Creates a new mountpoint.
///
/// If a mountpoint is already present at the same path, the function fails with [`errno::EINVAL`].
///
/// Arguments:
/// - `source` is the source of the mountpoint.
/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it automatically.
/// - `flags` are the mount flags.
/// - `target_path` is the path on which the filesystem is to be mounted.
/// - `target_location` is the location on which the filesystem is to be mounted on the parent
///   filesystem.
///
/// The function returns the ID of the newly created mountpoint.
pub fn create(
	source: MountSource,
	fs_type: Option<Arc<dyn FilesystemType>>,
	flags: u32,
	target_path: PathBuf,
	target_location: FileLocation,
) -> EResult<Arc<Mutex<MountPoint>>> {
	// TODO clean
	// PATH_TO_ID is locked first and during the whole function to prevent a race condition between
	// the locks of MOUNT_POINTS
	let mut path_to_id = LOC_TO_ID.lock();
	// If a mountpoint is already present at this location, error
	if path_to_id.get(&target_location).is_some() {
		return Err(errno!(EINVAL));
	}

	// TODO improve
	// ID allocation
	let id = {
		MOUNT_POINTS
			.lock()
			.iter()
			.map(|(i, _)| *i + 1)
			.max()
			.unwrap_or(0)
	};

	let mountpoint = Arc::new(Mutex::new(MountPoint::new(
		id,
		source,
		fs_type,
		flags,
		target_path,
		target_location.clone(),
	)?))?;

	// Insertion
	{
		let mut mount_points = MOUNT_POINTS.lock();
		mount_points.insert(id, mountpoint.clone())?;
		if let Err(e) = path_to_id.insert(target_location, id) {
			mount_points.remove(&id);
			return Err(e.into());
		}
	}

	Ok(mountpoint)
}

/// Removes the mountpoint at the given `target_location`.
///
/// Data is synchronized to the associated storage device, if any, before removing the mountpoint.
///
/// If the mountpoint doesn't exist, the function returns [`errno::EINVAL`].
///
/// If the mountpoint is busy, the function returns [`errno::EBUSY`].
pub fn remove(target_location: &FileLocation) -> EResult<()> {
	let mut loc_to_id = LOC_TO_ID.lock();
	let mut mount_points = MOUNT_POINTS.lock();

	let id = *loc_to_id.get(target_location).ok_or(errno!(EINVAL))?;
	let _mountpoint = mount_points.get(&id).ok_or(errno!(EINVAL))?;

	// TODO Check if busy (EBUSY)
	// TODO Check if another mount point is present in a subdirectory (EBUSY)

	// TODO sync fs

	loc_to_id.remove(target_location);
	mount_points.remove(&id);

	Ok(())
}

/// Returns the mountpoint with id `id`.
///
/// If it doesn't exist, the function returns `None`.
pub fn from_id(id: u32) -> Option<Arc<Mutex<MountPoint>>> {
	MOUNT_POINTS.lock().get(&id).cloned()
}

/// Returns the mountpoint that is mounted at the given `target_location`.
///
/// If it doesn't exist, the function returns `None`.
pub fn from_location(target_location: &FileLocation) -> Option<Arc<Mutex<MountPoint>>> {
	let loc_to_id = LOC_TO_ID.lock();
	let id = loc_to_id.get(target_location)?;
	from_id(*id)
}

/// Returns the location to start path resolution from.
///
/// If no the root mountpoint does not exist, the function panics.
pub fn root_location() -> FileLocation {
	// TODO cache?
	let Some(root_mp_mutex) = from_id(0) else {
		panic!("No root mountpoint!");
	};
	let root_mp = root_mp_mutex.lock();
	let root_inode = root_mp.get_filesystem().lock().get_root_inode();
	FileLocation::Filesystem {
		mountpoint_id: root_mp.get_id(),
		inode: root_inode,
	}
}
