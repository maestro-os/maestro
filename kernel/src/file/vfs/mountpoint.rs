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

use crate::{
	device,
	device::{DeviceID, DeviceType},
	file::{
		fs,
		fs::{Filesystem, FilesystemType},
		vfs,
		vfs::{EntryChild, ResolutionSettings},
		FileType,
	},
	sync::mutex::Mutex,
};
use core::fmt;
use utils::{
	collections::{
		hashmap::HashMap,
		path::{Path, PathBuf},
		string::String,
	},
	errno,
	errno::{AllocResult, EResult},
	ptr::arc::Arc,
	TryClone,
};

/// Permits mandatory locking on files.
pub const FLAG_MANDLOCK: u32 = 0b000000000001;
/// Do not update file (all kinds) access timestamps on the filesystem.
pub const FLAG_NOATIME: u32 = 0b000000000010;
/// Do not allow access to device files on the filesystem.
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

/// Value specifying the device from which a filesystem is mounted.
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum MountSource {
	/// The mountpoint is mounted from a device.
	Device(DeviceID),
	/// The mountpoint is bound to a virtual filesystem and thus isn't
	/// associated with any device.
	///
	/// The string value is the name of the source.
	NoDev(String),
}

impl MountSource {
	/// Creates a mount source from a dummy string.
	///
	/// `string` might be either a kernfs name or an absolute path.
	pub fn new(string: &[u8]) -> EResult<Self> {
		let path = Path::new(string)?;
		let result = vfs::get_file_from_path(path, &ResolutionSettings::kernel_follow());
		match result {
			Ok(ent) => {
				let stat = ent.stat();
				match stat.get_type() {
					Some(FileType::BlockDevice) => Ok(Self::Device(DeviceID {
						dev_type: DeviceType::Block,
						major: stat.dev_major,
						minor: stat.dev_minor,
					})),
					Some(FileType::CharDevice) => Ok(Self::Device(DeviceID {
						dev_type: DeviceType::Char,
						major: stat.dev_major,
						minor: stat.dev_minor,
					})),
					_ => Err(errno!(EINVAL)),
				}
			}
			Err(err) if err == errno!(ENOENT) => Ok(Self::NoDev(String::try_from(string)?)),
			Err(err) => Err(err),
		}
	}
}

impl TryClone for MountSource {
	fn try_clone(&self) -> AllocResult<Self> {
		Ok(match self {
			Self::Device(id) => Self::Device(*id),
			Self::NoDev(name) => Self::NoDev(name.try_clone()?),
		})
	}
}

impl fmt::Display for MountSource {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		match self {
			Self::Device(DeviceID {
				dev_type,
				major,
				minor,
			}) => write!(fmt, "dev({dev_type}:{major}:{minor})"),
			Self::NoDev(name) => write!(fmt, "{name}"),
		}
	}
}

/// The list of loaded filesystems associated with their respective sources.
static FILESYSTEMS: Mutex<HashMap<DeviceID, Arc<Filesystem>>> = Mutex::new(HashMap::new());

/// Returns the loaded filesystem with the given source `source`. If not loaded, the function loads
/// it.
///
/// Arguments:
/// - `source` is the source of the mountpoint.
/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it automatically.
/// - `target_path` is the path at which the filesystem is to be mounted.
/// - `readonly` tells whether the filesystem is mount in readonly.
fn get_fs(
	source: &MountSource,
	fs_type: Option<Arc<dyn FilesystemType>>,
	target_path: PathBuf,
	readonly: bool,
) -> EResult<Arc<Filesystem>> {
	match source {
		MountSource::Device(dev_id) => {
			let mut filesystems = FILESYSTEMS.lock();
			// If the filesystem is already loaded, return it
			if let Some(fs) = filesystems.get(dev_id) {
				return Ok(fs.clone());
			}
			// Else, load it
			let dev = device::get(dev_id).ok_or_else(|| errno!(ENODEV))?;
			let fs_type = match fs_type {
				Some(f) => f,
				None => fs::detect(Arc::as_ref(dev.get_io()))?,
			};
			let ops =
				fs_type.load_filesystem(Some(dev.get_io().clone()), target_path, readonly)?;
			let fs = Filesystem::new(dev_id.get_device_number(), ops)?;
			// Insert new filesystem into filesystems list
			filesystems.insert(*dev_id, fs.clone())?;
			Ok(fs)
		}
		MountSource::NoDev(name) => {
			let fs_type = match fs_type {
				Some(f) => f,
				None => fs::get_type(name).ok_or_else(|| errno!(ENODEV))?,
			};
			let ops = fs_type.load_filesystem(None, target_path, readonly)?;
			let fs = Filesystem::new(0, ops)?;
			Ok(fs)
		}
	}
}

/// A mount point, allowing to attach a filesystem to a directory on the VFS.
#[derive(Debug)]
pub struct MountPoint {
	/// Mount flags.
	pub flags: u32,
	/// The source of the mountpoint.
	pub source: MountSource,
	/// The filesystem associated with the mountpoint.
	pub fs: Arc<Filesystem>,
	/// The root entry of the mountpoint.
	pub root_entry: Arc<vfs::Entry>,
}

impl Drop for MountPoint {
	fn drop(&mut self) {
		// If not associated with a device, stop
		let MountSource::Device(dev_id) = &self.source else {
			return;
		};
		let mut filesystems = FILESYSTEMS.lock();
		let Some(fs) = filesystems.get(dev_id) else {
			return;
		};
		/*
		 * Remove the associated filesystem if this was the last reference to it.
		 *
		 * the current instance + FILESYSTEMS = `2`
		 */
		if Arc::strong_count(fs) <= 2 {
			filesystems.remove(dev_id);
		}
	}
}

/// The list of mountpoints with their respective ID.
pub static MOUNT_POINTS: Mutex<HashMap<*const vfs::Entry, Arc<MountPoint>>> =
	Mutex::new(HashMap::new());

/// Creates a new mountpoint.
///
/// If a mountpoint is already present at the same path, the function fails with [`errno::EINVAL`].
///
/// Arguments:
/// - `source` is the source of the mountpoint
/// - `fs_type` is the filesystem type. If `None`, the function tries to detect it automatically
/// - `flags` are the mount flags
/// - `target` is the target directory. If `None`, the mountpoint is root
///
/// The function returns the root VFS entry of the mountpoint.
pub fn create(
	source: MountSource,
	fs_type: Option<Arc<dyn FilesystemType>>,
	flags: u32,
	target: Option<Arc<vfs::Entry>>,
) -> EResult<Arc<vfs::Entry>> {
	// Get filesystem
	let (target_path, name, parent) = match target {
		Some(target) => (
			vfs::Entry::get_path(&target)?,
			target.name.try_clone()?,
			target.parent.clone(),
		),
		None => (PathBuf::root()?, String::new(), None),
	};
	let fs = get_fs(&source, fs_type, target_path, flags & FLAG_RDONLY != 0)?;
	let mut mps = MOUNT_POINTS.lock();
	// TODO get root node from cache if present instead
	// Get filesystem root node
	let root = fs.ops.root(fs.clone())?;
	fs.node_insert(root.clone())?;
	// Create an entry for the root of the mountpoint
	let root_entry = Arc::new(vfs::Entry {
		name,
		parent: parent.clone(),
		children: Default::default(),
		node: Some(root),
	})?;
	// Create mountpoint
	let mountpoint = Arc::new(MountPoint {
		flags,
		source,
		fs,
		root_entry: root_entry.clone(),
	})?;
	// If the next insertion fails, this will be undone by the implementation of `Drop`
	mps.insert(Arc::as_ptr(&root_entry), mountpoint)?;
	// Replace `target` with the mountpoint's root in the tree
	if let Some(target_parent) = &parent {
		target_parent
			.children
			.lock()
			.insert(EntryChild(root_entry.clone()))?;
	}
	Ok(root_entry)
}

/// Removes the mountpoint at the given `target` entry.
///
/// Data is synchronized to the associated storage device, if any, before removing the mountpoint.
///
/// If `target` is not a mountpoint, the function returns [`errno::EINVAL`].
///
/// If the mountpoint is busy, the function returns [`errno::EBUSY`].
pub fn remove(target: Arc<vfs::Entry>) -> EResult<()> {
	// TODO Check if another mount point is present in a subdirectory (EBUSY)
	// TODO Check if busy (EBUSY)
	// Detach entry from parent
	let Some(parent) = &target.parent else {
		// Cannot unmount root filesystem
		return Err(errno!(EINVAL));
	};
	parent.children.lock().remove(target.name.as_bytes());
	// TODO release node and children
	MOUNT_POINTS.lock().remove(&Arc::as_ptr(&target));
	Ok(())
}

/// Returns the mountpoint for the root entry `ent`.
///
/// If `ent` is not associated to a mountpoint, the function returns `None`.
pub fn from_entry(ent: &vfs::Entry) -> Option<Arc<MountPoint>> {
	MOUNT_POINTS.lock().get(&(ent as _)).cloned()
}
