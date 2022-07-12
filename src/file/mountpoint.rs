//! A mount point is a directory in which a filesystem is mounted.

use crate::device::Device;
use crate::errno::Errno;
use crate::file::File;
use crate::file::fcache;
use crate::file::fs::Filesystem;
use crate::file::fs::FilesystemType;
use crate::file::fs;
use crate::util::DummyIO;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
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
	/// The mountpoint is mounted to a kernfs.
	KernFS(String),
}

impl MountSource {
	/// Creates a mount source from a dummy string.
	/// The string `string` might be either a kernfs name, a relative path or an absolute path.
	/// `cwd` is the current working directory.
	pub fn from_str(string: &[u8], cwd: Path) -> Result<Self, Errno> {
		let path = Path::from_str(string, true)?;
		let path = cwd.concat(&path)?;
		let result = {
			let mutex = fcache::get();
			let guard = mutex.lock();
			let fcache = guard.get_mut().as_mut().unwrap();

			fcache.get_file_from_path(&path, 0, 0, true)
		};

		match result {
			Ok(file) => Ok(Self::File(file)),
			Err(err) if err == errno!(ENOENT) => Ok(Self::KernFS(String::from(string)?)),
			Err(err) => Err(err),
		}
	}

	/// Returns the IO interface for the mount source.
	pub fn get_io(&self) -> Result<SharedPtr<dyn IO>, Errno> {
		match self {
			Self::Device(dev) => Ok(dev.clone() as _),
			Self::File(file) => Ok(file.clone() as _),
			Self::KernFS(_) => Ok(SharedPtr::new(DummyIO {})? as _),
		}
	}
}

impl Eq for MountSource {}

impl PartialEq for MountSource {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Device(dev0), Self::Device(dev1)) => todo!(), // TODO
			(Self::File(file0), Self::File(file1)) => todo!(), // TODO
			(Self::KernFS(fs0), Self::KernFS(fs1)) => fs0 == fs1,
		}
	}
}

/// The list of loaded filesystems associated with their respective ID.
static FILESYSTEMS: Mutex<Vec<(u32, SharedPtr<dyn Filesystem>)>>
	= Mutex::new(Vec::new());

/// Loads a filesystem.
/// `io` is the I/O interface to the storage device.
/// `path` is the path to the directory on which the filesystem is mounted.
/// `fs_type` is the type of the filesystem to be loaded.
/// `readonly` tells whether the filesystem is mount in readonly.
/// On success, the function returns the ID of the loaded filesystem.
fn load_fs(io: &mut dyn IO, path: Path, fs_type: &dyn FilesystemType, readonly: bool)
	-> Result<u32, Errno> {
	// TODO Alloc id
	let fs = fs_type.load_filesystem(io, fs_id, path, readonly)?;

	let guard = FILESYSTEMS.lock();
	let container = guard.get_mut();

	let index = match container.binary_search_by(| (i, _) | i.cmp(&fs_id)) {
		Ok(i) | Err(i) => i,
	};

	container.insert(index, fs)?;
	Ok(id)
}

/// Returns the filesystem with the given ID `id`.
fn get_fs(id: u32) -> Option<SharedPtr<dyn Filesystem>> {
	let guard = FILESYSTEMS.lock();
	let container = guard.get_mut();

	let index = container.binary_search_by(| (i, _) | i.cmp(&id)).ok()?;
	Some(container[index].1.clone())
}

/// Structure representing a mount point.
pub struct MountPoint {
	/// The source of the mountpoint.
	source: MountSource,
	/// The ID of the filesystem associated with the mountpoint.
	fs_id: u32,

	/// Mount flags.
	flags: u32,
	/// The path to the mount directory.
	path: Path,
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
		let io_mutex = source.get_io()?;
		let io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Tells whether the filesystem will be mounted in read-only
		let readonly = flags & FLAG_RDONLY != 0;

		// Getting the filesystem type
		let fs_type_mutex = match fs_type {
			Some(fs_type) => fs_type,
			None => fs::detect(io)?,
		};
		let fs_type_guard = fs_type_mutex.lock();
		let fs_type = fs_type_guard.get();

		// Loading the filesystem
		// TODO If the filesystem is already loaded, use the same instance instead
		let fs_id = load_fs(io, path.failable_clone()?, fs_type, readonly)?;

		Ok(Self {
			source,
			fs_id,

			flags,
			path,
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
	pub fn get_filesystem(&mut self) -> SharedPtr<dyn Filesystem> {
		get_fs(self.fs_id).unwrap()
	}

	/// Tells whether the mountpoint's is mounted in read-only.
	#[inline(always)]
	pub fn is_readonly(&self) -> bool {
		self.flags & FLAG_RDONLY != 0
	}
}

/// The list of mountpoints with their respective ID.
static MOUNT_POINTS: Mutex<Vec<(u32, Path, SharedPtr<MountPoint>)>> = Mutex::new(Vec::new());

/// Registers a new mountpoint `mountpoint`. If a mountpoint is already present at the same path,
/// the function fails.
pub fn register(mountpoint: MountPoint) -> Result<SharedPtr<MountPoint>, Errno> {
	let guard = MOUNT_POINTS.lock();
	let container = guard.get_mut();

	let shared_ptr = SharedPtr::new(mountpoint)?;
	container.insert(path, shared_ptr.clone())?;
	Ok(shared_ptr)
}

/// Returns the deepest mountpoint in the path `path`. If no mountpoint is in the path, the
/// function returns None.
pub fn get_deepest(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let guard = MOUNT_POINTS.lock();
	let container = guard.get_mut();

	let mut max: Option<SharedPtr<MountPoint>> = None;
	for (_, mount_path, m) in container.iter() {
		if let Some(max) = max.as_mut() {
			let max_guard = max.lock();
			let max_path = max_guard.get().get_path();

			if max_path.get_elements_count() >= mount_path.get_elements_count() {
				continue;
			}
		}

		if path.begins_with(mount_path) {
			max = Some(m.clone());
		}
	}

	max
}

/// Returns the mountpoint with id `id`. If it doesn't exist, the function returns None.
pub fn from_id(id: u32) -> Option<SharedPtr<MountPoint>> {
	let guard = MOUNT_POINTS.lock();
	let container = guard.get_mut();

	let index = container.binary_search_by(| (i, _, _) | i.cmp(&id)).ok()?;
	Some(container[index].2.clone())
}

/// Returns the mountpoint with path `path`. If it doesn't exist, the function returns None.
pub fn from_path(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let guard = MOUNT_POINTS.lock();
	let container = guard.get_mut();

	Some(container.iter()
		.filter(| (_, p, _ )| p == path)
		.next()?
		.clone())
}
