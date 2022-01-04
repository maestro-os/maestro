//! A mount point is a directory in which a filesystem is mounted.

use crate::device::Device;
use crate::errno::Errno;
use crate::file::File;
use crate::file::fcache;
use crate::file::fs::Filesystem;
use crate::file::fs::FilesystemType;
use crate::file::fs;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;
use super::path::Path;

/// Permits mandatory locking on files.
const FLAG_MANDLOCK: u32    = 0b000000000001;
/// Do not update file (all kinds) access timestamps on the filesystem.
const FLAG_NOATIME: u32     = 0b000000000010;
/// Do not allows access to device files on the filesystem.
const FLAG_NODEV: u32       = 0b000000000100;
/// Do not update directory access timestamps on the filesystem.
const FLAG_NODIRATIME: u32  = 0b000000001000;
/// Do not allow files on the filesystem to be executed.
const FLAG_NOEXEC: u32      = 0b000000010000;
/// Ignore setuid and setgid flags on the filesystem.
const FLAG_NOSUID: u32      = 0b000000100000;
/// Mounts the filesystem in read-only.
const FLAG_RDONLY: u32      = 0b000001000000;
/// TODO doc
const FLAG_REC: u32         = 0b000010000000;
/// Update atime only if less than or equal to mtime or ctime.
const FLAG_RELATIME: u32    = 0b000100000000;
/// Suppresses certain warning messages in the kernel logs.
const FLAG_SILENT: u32      = 0b001000000000;
/// Always update the last access time when files on this filesystem are accessed. Overrides
/// NOATIME and RELATIME.
const FLAG_STRICTATIME: u32 = 0b010000000000;
/// Makes writes on this filesystem synchronous.
const FLAG_SYNCHRONOUS: u32 = 0b100000000000;

// TODO When removing a mountpoint, return an error if another mountpoint is present in a subdir

/// Enumeration of mount sources.
pub enum MountSource {
	/// The mountpoint is mounted from a device.
	Device(SharedPtr<Device>),
	/// The mountpoint is mounted from a file.
	File(SharedPtr<File>),
}

impl MountSource {
	/// Creates a mount source from a dummy string.
	pub fn from_str(string: &[u8]) -> Result<Self, Errno> {
		// TODO Handle kernfs

		let path = Path::from_str(string, true)?;
		let file = {
			let mutex = fcache::get();
			let mut guard = mutex.lock();
			let fcache = guard.get_mut().as_mut().unwrap();

			fcache.get_file_from_path(&path)?
		};

		Ok(Self::File(file))
	}

	/// Returns the IO interface for the mount source.
	pub fn get_io(&self) -> SharedPtr<dyn IO> {
		match self {
			Self::Device(dev) => dev.clone() as _,
			Self::File(file) => file.clone() as _,
		}
	}
}

/// Structure representing a mount point.
pub struct MountPoint {
	/// The source of the mountpoint.
	source: MountSource,

	/// Mount flags.
	flags: u32,
	/// The path to the mount directory.
	path: Path,

	/// An instance of the filesystem associated with the mountpoint.
	filesystem: Box<dyn Filesystem>,
}

impl MountPoint {
	/// Creates a new instance.
	/// `source` is the source of the mountpoint.
	/// `fs_type` is the filesystem type. If None, the function tries to detect it automaticaly.
	/// `flags` are the mount flags.
	/// `path` is the path on which the filesystem is to be mounted.
	pub fn new(source: MountSource, fs_type: Option<SharedPtr<dyn FilesystemType>>, flags: u32,
		path: Path) -> Result<Self, Errno> {
		// Getting the I/O interface
		let io_mutex = source.get_io();
		let mut io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Tells whether the filesystem will be mounted in read-only
		let readonly = flags & FLAG_RDONLY != 0;

		// Getting the filesystem type
		let fs_type_ptr = match fs_type {
			Some(fs_type) => fs_type,
			None => fs::detect(io)?,
		};
		let fs_type_guard = fs_type_ptr.lock();
		let fs_type = fs_type_guard.get();

		// Loading the filesystem
		let filesystem = fs_type.load_filesystem(io, path.failable_clone()?, readonly)?;

		Ok(Self {
			source,

			flags,
			path,

			filesystem,
		})
	}

	/// Returns the source of the mountpoint.
	#[inline(always)]
	pub fn get_source(&self) -> &MountSource {
		&self.source
	}

	/// Returns the mountpoint's flags.
	#[inline(always)]
	pub fn get_flags(&self) -> u32 {
		self.flags
	}

	/// Returns a reference to the path where the filesystem is mounted.
	#[inline(always)]
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Returns a mutable reference to the filesystem associated with the device.
	#[inline(always)]
	pub fn get_filesystem(&mut self) -> &mut dyn Filesystem {
		self.filesystem.as_mut()
	}

	/// Tells whether the mountpoint's filesystem is mounted in read-only.
	#[inline(always)]
	pub fn is_readonly(&self) -> bool {
		self.flags & FLAG_RDONLY != 0 || self.filesystem.is_readonly()
	}

	/// Tells the kernel whether it must cache files.
	#[inline(always)]
	pub fn must_cache(&self) -> bool {
		self.filesystem.must_cache()
	}
}

/// The list of mountpoints.
static MOUNT_POINTS: Mutex<HashMap<Path, SharedPtr<MountPoint>>> = Mutex::new(HashMap::new());

/// Registers a new mountpoint `mountpoint`. If a mountpoint is already present at the same path,
/// the function fails.
pub fn register(mountpoint: MountPoint) -> Result<SharedPtr<MountPoint>, Errno> {
	let mut guard = MOUNT_POINTS.lock();
	let container = guard.get_mut();

	let path = mountpoint.get_path().failable_clone()?;

	let shared_ptr = SharedPtr::new(mountpoint)?;
	container.insert(path, shared_ptr.clone())?;
	Ok(shared_ptr)
}

/// Returns the deepest mountpoint in the path `path`. If no mountpoint is in the path, the
/// function returns None.
pub fn get_deepest(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let mut guard = MOUNT_POINTS.lock();
	let container = guard.get_mut();

	let mut max: Option<SharedPtr<MountPoint>> = None;
	for m in container.iter() {
		let mount_guard = m.lock();
		let mount_path = mount_guard.get().get_path();

		if let Some(max) = max.as_mut() {
			let max_guard = max.lock();
			let max_path = max_guard.get().get_path();

			if max_path.get_elements_count() >= mount_path.get_elements_count() {
				continue;
			}
		}

		if path.begins_with(mount_path) {
			drop(mount_guard);
			max = Some(m.clone());
		}
	}

	max
}

/// Returns the mountpoint with path `path`. If it doesn't exist, the function returns None.
pub fn from_path(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let mut guard = MOUNT_POINTS.lock();
	let container = guard.get_mut();

	Some(container.get(path)?.clone())
}
