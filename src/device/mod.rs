//! This module handles device and buses.
//! A device file is an interface with a device of the system, which can be internal or external,
//! or even virtual such as a TTY.

pub mod bar;
pub mod bus;
pub mod default;
pub mod driver;
pub mod id;
pub mod keyboard;
pub mod manager;
pub mod network;
pub mod serial;
pub mod storage;

use core::ffi::c_void;
use crate::device::manager::DeviceManager;
use crate::errno::Errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Mode;
use crate::file::fcache::FCache;
use crate::file::fcache;
use crate::file::path::Path;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;
use keyboard::KeyboardManager;
use storage::StorageManager;

/// Enumeration representing the type of the device.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DeviceType {
	/// A block device.
	Block,
	/// A char device.
	Char,
}

/// Trait providing a interface for device I/O.
pub trait DeviceHandle: IO {
	/// Performs an ioctl operation on the device.
	fn ioctl(&mut self, request: u32, argp: *const c_void) -> Result<u32, Errno>;
}

/// Structure representing a device, either a block device or a char device. Each device has a
/// major and a minor number.
pub struct Device {
	/// The major number.
	major: u32,
	/// The minor number.
	minor: u32,

	/// The path to the device file.
	path: Path,
	/// The file's mode.
	mode: Mode,
	/// The type of the device.
	type_: DeviceType,

	/// The object handling the device I/O.
	handle: Box<dyn DeviceHandle>,
}

impl Device {
	/// Creates a new instance.
	/// `major` and `minor` are the major and minor numbers of the device.
	/// `type_` is the type of the device.
	/// `handle` is the handle for I/O operations.
	pub fn new<H: 'static + DeviceHandle>(major: u32, minor: u32, path: Path, mode: Mode,
		type_: DeviceType, handle: H) -> Result<Self, Errno> {
		Ok(Self {
			major,
			minor,

			path,
			mode,
			type_,

			handle: Box::new(handle)?,
		})
	}

	/// Returns the major number.
	#[inline]
	pub fn get_major(&self) -> u32 {
		self.major
	}

	/// Returns the minor number.
	#[inline]
	pub fn get_minor(&self) -> u32 {
		self.minor
	}

	/// Returns the path to the device file.
	#[inline]
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Returns the device file's mode.
	#[inline]
	pub fn get_mode(&self) -> Mode {
		self.mode
	}

	/// Returns the type of the device.
	#[inline]
	pub fn get_type(&self) -> DeviceType {
		self.type_
	}

	/// Returns the device number.
	#[inline]
	pub fn get_device_number(&self) -> u64 {
		id::makedev(self.major, self.minor)
	}

	/// Returns the handle of the device for I/O operations.
	#[inline]
	pub fn get_handle(&mut self) -> &mut dyn DeviceHandle {
		self.handle.as_mut()
	}

	/// Creates the directories necessary to reach path `path`. On success, the function returns
	/// the number of created directories (without the directories that already existed).
	/// If relative, the path is taken from the root.
	fn create_dirs(fcache: &mut FCache, path: &Path) -> Result<usize, Errno> {
		let mut path = Path::root().concat(path)?;
		path.reduce()?;
		let mut p = Path::root();

		let mut created_count = 0;
		for i in 0..path.get_elements_count() {
			p.push(path[i].failable_clone()?)?;

			if fcache.get_file_from_path(&p).is_err() {
				let dir = File::new(p[i].failable_clone()?, FileContent::Directory(Vec::new()), 0,
					0, 0o755)?;
				fcache.create_file(&p.range_to(..i)?, dir)?;

				created_count += 1;
			}
		}

		Ok(created_count)
	}

	/// Removes the file at path `path` and its subfiles recursively if it's a directory.
	/// If relative, the path is taken from the root.
	fn remove_recursive(_fcache: &mut FCache, path: &Path) -> Result<(), Errno> {
		let mut path = Path::root().concat(path)?;
		path.reduce()?;

		// TODO
		todo!();
	}

	// TODO Put file creation on the userspace side?
	/// Creates the device file associated with the structure. If the file already exist, the
	/// function does nothing.
	pub fn create_file(&mut self) -> Result<(), Errno> {
		let file_content = match self.type_ {
			DeviceType::Block => FileContent::BlockDevice {
				major: self.major,
				minor: self.minor,
			},

			DeviceType::Char => FileContent::CharDevice {
				major: self.major,
				minor: self.minor,
			},
		};

		let path_len = self.path.get_elements_count();
		let filename = self.path[path_len - 1].failable_clone()?;

		// Locking the files' cache
		let mutex = fcache::get();
		let mut guard = mutex.lock();
		let files_cache = guard.get_mut();

		// Tells whether the file already exists
		let file_exists = files_cache.as_mut().unwrap().get_file_from_path(&self.path).is_ok();

		if !file_exists {
			// Creating the directories in which the device file is located
			let mut dir_path = self.path.failable_clone()?;
			dir_path.pop();
			Self::create_dirs(files_cache.as_mut().unwrap(), &dir_path)?;

			let file = File::new(filename, file_content, 0, 0, self.mode)?;

			// TODO Cancel directories creation on fail
			// Creating the device file
			files_cache.as_mut().unwrap().create_file(&dir_path, file)?;
		}

		Ok(())
	}

	/// If exists, removes the device file. iF the file doesn't exist, the function does nothing.
	pub fn remove_file(&mut self) {
		let mutex = fcache::get();
		let mut guard = mutex.lock();
		let files_cache = guard.get_mut();

		if let Ok(file) = files_cache.as_mut().unwrap().get_file_from_path(&self.path) {
			let mut guard = file.lock();
			guard.get_mut().unlink();
		}
	}
}

impl IO for Device {
	fn get_size(&self) -> u64 {
		self.handle.get_size()
	}

	fn read(&self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		self.handle.read(offset, buff)
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		self.handle.write(offset, buff)
	}
}

impl Drop for Device {
	fn drop(&mut self) {
		self.remove_file();
	}
}

/// The list of registered block devices.
static BLOCK_DEVICES: Mutex<Vec<SharedPtr<Device>>> = Mutex::new(Vec::new());
/// The list of registered block devices.
static CHAR_DEVICES: Mutex<Vec<SharedPtr<Device>>> = Mutex::new(Vec::new());

/// Registers the given device. If the minor/major number is already used, the function fails.
/// The function *doesn't* create the device file.
pub fn register_device(device: Device) -> Result<(), Errno> {
	let mut guard = match device.get_type() {
		DeviceType::Block => BLOCK_DEVICES.lock(),
		DeviceType::Char => CHAR_DEVICES.lock(),
	};
	let container = guard.get_mut();

	let device_number = device.get_device_number();
	let index = container.binary_search_by(| d | {
		let dn = unsafe {
			d.get_mut().get_mut_payload()
		}.get_device_number();

		device_number.cmp(&dn)
	});
	let index = match index {
		Ok(i) => i,
		Err(i) => i,
	};

	container.insert(index, SharedPtr::new(device)?)
}

// TODO Function to remove a device

/// Returns a mutable reference to the device with the given major number, minor number and type.
/// If the device doesn't exist, the function returns None.
pub fn get_device(type_: DeviceType, major: u32, minor: u32) -> Option<SharedPtr<Device>> {
	let mut guard = match type_ {
		DeviceType::Block => BLOCK_DEVICES.lock(),
		DeviceType::Char => CHAR_DEVICES.lock(),
	};
	let container = guard.get_mut();

	let device_number = id::makedev(major, minor);
	let index = container.binary_search_by(| d | {
		let dn = unsafe {
			d.get_mut().get_mut_payload()
		}.get_device_number();

		device_number.cmp(&dn)
	});

	if let Ok(i) = index {
		Some(container[i].clone())
	} else {
		None
	}
}

/// Returns the device with the given path `path`.
/// This function is `O(n)` in time.
/// If no device with the given path is found, the function returns None.
pub fn get_by_path(path: &Path) -> Option<SharedPtr<Device>> {
	{
		let mut block_guard = BLOCK_DEVICES.lock();
		let block_container = block_guard.get_mut();

		for i in 0..block_container.len() {
			let dev_guard = block_container[i].lock();
			let dev = dev_guard.get();

			if dev.get_path() == path {
				drop(dev_guard);
				return Some(block_container[i].clone());
			}
		}
	}

	{
		let mut char_guard = CHAR_DEVICES.lock();
		let char_container = char_guard.get_mut();

		for i in 0..char_container.len() {
			let dev_guard = char_container[i].lock();
			let dev = dev_guard.get();

			if dev.get_path() == path {
				drop(dev_guard);
				return Some(char_container[i].clone());
			}
		}
	}

	None
}

/// Initializes devices management.
pub fn init() -> Result<(), Errno> {
	let mut keyboard_manager = KeyboardManager::new();
	keyboard_manager.legacy_detect()?;
	manager::register_manager(keyboard_manager)?;

	let storage_manager = StorageManager::new()?;
	//storage_manager.legacy_detect()?;
	manager::register_manager(storage_manager)?;

	bus::detect()?;

	// Testing disk I/O (if enabled)
	#[cfg(config_debug_storagetest)]
	{
		// Getting back the storage manager since it has been moved
		let storage_manager = manager::get_by_name("storage").unwrap();
		let storage_manager = unsafe {
			storage_manager.get_mut().unwrap().get_mut_payload()
		};
		let storage_manager = unsafe {
			&mut *(storage_manager as *mut _ as *mut StorageManager)
		};

		storage_manager.test();
	}

	Ok(())
}
