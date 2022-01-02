//! A mount point is a directory in which a filesystem is mounted.

use crate::device::Device;
use crate::device::DeviceType;
//use crate::device;
use crate::errno::Errno;
//use crate::file::File;
use crate::file::fs::Filesystem;
use crate::file::fs::FilesystemType;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::ptr::SharedPtr;
//use super::fs;
use super::path::Path;

/// TODO doc
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
	/// The source of the mountpoint.
	source: String,

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
	pub fn new(source: String, fs_type: Option<SharedPtr<dyn FilesystemType>>, flags: u32,
		path: Path) -> Result<Self, Errno> {
		// TODO Support kernfs
		/*let source_path = Path::from_str(source.as_bytes(), true)?;

		{
			let fcache = file::get_files_caches().lock(false).get_mut();
			let source_mutex = fcache.get_file_from_path();
		};

		let fs_type_ptr = fs_type.or(fs::detect(source)?);
		let fs_type_guard = fs_type_ptr.lock(true);
		let fs_type = fs_type_guard.get();
		let filesystem = fs_type.load_filesystem(source, &path)?;

		Ok(Self {
			source,

			flags,
			path,

			filesystem,
		})*/
		todo!();
	}

	/// Returns the source of the mountpoint.
	#[inline(always)]
	pub fn get_source(&self) -> &String {
		&self.source
	}

	/// Returns a reference to the mounted device.
	#[inline(always)]
	pub fn get_device(&self) -> SharedPtr<Device> {
		device::get_device(self.device_type, self.major, self.minor).unwrap()
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
	let mut guard = MOUNT_POINTS.lock(true);
	let container = guard.get_mut();
	let shared_ptr = SharedPtr::new(mountpoint)?;
	container.push(shared_ptr.clone())?;
	Ok(shared_ptr)
}

/// Returns the deepest mountpoint in the path `path`. If no mountpoint is in the path, the
/// function returns None.
pub fn get_deepest(path: &Path) -> Option<SharedPtr<MountPoint>> {
	let mut guard = MOUNT_POINTS.lock(true);
	let container = guard.get_mut();

	let mut max: Option<SharedPtr<MountPoint>> = None;
	for i in 0..container.len() {
		let m = &mut container[i];
		let mount_guard = m.lock(true);
		let mount_path = mount_guard.get().get_path();

		if let Some(max) = max.as_mut() {
			let max_guard = max.lock(true);
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
	// TODO
}
